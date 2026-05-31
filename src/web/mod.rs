//! Web 后端:把交互式 Agent 暴露为浏览器 UI。
//!
//! 架构:
//! - Agent 循环跑在专用 std::thread(内置 current-thread tokio runtime 驱动 async 后端),
//!   通过 `mpsc`(web → agent 输入)与 `broadcast`(agent → web 事件)与 axum 通信。
//! - axum 在主 runtime 提供 HTTP(配置/元信息/静态资源)与 WebSocket(事件流 + 用户输入)。
//! - 复用 `Agent::llm_turn`;`WebView` 实现 `TurnView`,把流式增量与命令结果转为 `WebEvent`。

use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::agent::shell::{CommandOutput, ShellSession};
use crate::agent::{Agent, TurnOutcome, TurnView};
use crate::block::Block;
use crate::config::{self, Config, Scope};
use crate::context::RuntimeContext;
use crate::error::{DoitError, Result};
use crate::session::Session;

/// 嵌入前端构建产物(web/build)。构建:`cd web && bun run build`。
#[derive(rust_embed::RustEmbed)]
#[folder = "web/build"]
struct Assets;

/// 下行事件(WebSocket, JSON tagged)。
#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebEvent {
    /// LLM 思维链流式增量
    Reasoning { delta: String },
    /// LLM 自由文本流式增量
    Content { delta: String },
    /// 一次流式回合结束(前端定稿当前块)
    StreamEnd,
    /// 一条命令执行完成(含概述、命令、输出)
    CommandResult {
        narration: String,
        command: String,
        output: String,
        exit_code: i32,
        is_exit: bool,
    },
    /// Agent 发起 doit prompt,等待用户回复
    Prompt { message: String },
    /// 顶层回合:等待用户下一条消息
    AwaitUser,
    /// 回显用户输入到对话流
    UserInput { text: String },
    /// 会话结束(doit exit)
    SessionEnded,
}

/// 事件出口:同一把锁内追加历史并广播,保证新连接「回放历史 + 实时流」无缝衔接。
#[derive(Clone)]
struct EventSink {
    tx: broadcast::Sender<WebEvent>,
    history: Arc<Mutex<Vec<WebEvent>>>,
}

impl EventSink {
    fn emit(&self, ev: WebEvent) {
        let mut h = self.history.lock().unwrap();
        h.push(ev.clone());
        let _ = self.tx.send(ev); // 无订阅者时返回 Err,事件仍在历史中,忽略
    }
}

/// Web 视图:实现 `TurnView`,把 LLM 流式增量与命令结果转为 `WebEvent`;
/// 并通过 `input_rx` 阻塞获取用户输入(顶层回合与 doit prompt 共用同一输入流)。
pub struct WebView {
    sink: EventSink,
    input_rx: mpsc::Receiver<String>,
    narration: String,
}

impl WebView {
    fn recv_input(&self) -> Result<String> {
        self.input_rx
            .recv()
            .map_err(|_| DoitError::internal("web input channel closed"))
    }
}

impl TurnView for WebView {
    fn on_stream(&mut self, ev: crate::backend::StreamEvent) {
        use crate::backend::StreamEvent;
        match ev {
            StreamEvent::Reasoning(s) => self.sink.emit(WebEvent::Reasoning { delta: s.into() }),
            StreamEvent::Content(s) => self.sink.emit(WebEvent::Content { delta: s.into() }),
            StreamEvent::Narration(s) => self.narration.push_str(s),
            // 第一版不实时流命令文本:命令在 on_command 一次性给出
            StreamEvent::Command(_) => {}
        }
    }

    fn on_stream_end(&mut self) {
        self.sink.emit(WebEvent::StreamEnd);
    }

    fn on_command(&mut self, cmd: &str, out: &CommandOutput, is_exit: bool) {
        let narration = std::mem::take(&mut self.narration);
        self.sink.emit(WebEvent::CommandResult {
            narration: narration.trim().to_string(),
            command: cmd.to_string(),
            output: out.output.clone(),
            exit_code: out.exit_code,
            is_exit,
        });
    }

    fn handle_prompt(&mut self, cmd: &str) -> Option<Result<String>> {
        self.narration.clear(); // doit prompt 不展示概述
        let message = parse_prompt_message(cmd);
        self.sink.emit(WebEvent::Prompt { message });
        let reply = self.recv_input();
        if let Ok(text) = &reply {
            self.sink.emit(WebEvent::UserInput { text: text.clone() });
        }
        Some(reply)
    }
}

/// 从 `doit prompt '<message>'` 中解析出提示消息(LLM 使用单个位置参数)。
fn parse_prompt_message(cmd: &str) -> String {
    match shell_words::split(cmd) {
        // ["doit", "prompt", message, ...]
        Ok(parts) if parts.len() >= 3 => parts[2].clone(),
        _ => cmd
            .trim_start()
            .strip_prefix("doit prompt")
            .unwrap_or(cmd)
            .trim()
            .trim_matches(['\'', '"'])
            .to_string(),
    }
}

/// axum 共享状态。
struct AppState {
    sink: EventSink,
    input_tx: mpsc::Sender<String>,
}

/// 启动 Web 服务:在专用线程驱动 Agent,在主 runtime 跑 axum。
pub async fn run(ctx: RuntimeContext, host: &str, port: u16) -> Result<()> {
    let config = ctx.config.clone();
    let (event_tx, _rx) = broadcast::channel::<WebEvent>(1024);
    let history: Arc<Mutex<Vec<WebEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let (input_tx, input_rx) = mpsc::channel::<String>();
    let sink = EventSink {
        tx: event_tx,
        history,
    };

    // Agent 线程:专用 std::thread + current-thread runtime,与 async 服务隔离阻塞 PTY 操作
    let view = WebView {
        sink: sink.clone(),
        input_rx,
        narration: String::new(),
    };
    let agent_config = config.clone();
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("agent runtime: {e}");
                return;
            }
        };
        let agent = Agent::from_config(&agent_config);
        let cwd = std::env::current_dir().unwrap_or_default();
        let session = match Session::create(&cwd, &agent_config.model.name) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("session: {e}");
                return;
            }
        };
        if let Err(e) = rt.block_on(drive(agent, ctx, session, view)) {
            eprintln!("agent loop error: {e}");
        }
    });

    let state = Arc::new(AppState { sink, input_tx });
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/api/config", get(get_config).put(put_config))
        .route("/api/meta", get(get_meta))
        .fallback(static_handler)
        .with_state(state);

    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| DoitError::io(e, format!("bind {addr}")))?;
    let local = listener
        .local_addr()
        .map_err(|e| DoitError::io(e, "local_addr"))?;
    println!("doit web → http://{local}");
    axum::serve(listener, app)
        .await
        .map_err(|e| DoitError::internal(format!("serve: {e}")))?;
    Ok(())
}

/// web 驱动循环:顶层用户回合与 LLM 回合交替,复用 `llm_turn`。
async fn drive(
    agent: Agent,
    ctx: RuntimeContext,
    mut session: Session,
    mut view: WebView,
) -> Result<()> {
    let system = agent.generate_system_prompt(&ctx, true).await?;
    session.append(Block::System {
        seq: session.next_seq(),
        content: system,
    })?;
    // 非交互 shell:仅捕获命令输出(经事件推送给前端),不接管真实终端、不转发
    let mut shell = ShellSession::spawn(&session.dir, false, false)?;

    loop {
        view.sink.emit(WebEvent::AwaitUser);
        let input = match view.recv_input() {
            Ok(i) => i.trim().to_string(),
            Err(_) => break, // 输入通道关闭(服务退出)
        };
        if input.is_empty() {
            continue;
        }
        view.sink.emit(WebEvent::UserInput {
            text: input.clone(),
        });
        session.append(Block::User {
            seq: session.next_seq(),
            content: input,
        })?;

        loop {
            match agent.llm_turn(&mut session, &mut shell, &mut view).await? {
                TurnOutcome::Continue => continue,
                TurnOutcome::AwaitUser => break,
                TurnOutcome::Exit => {
                    view.sink.emit(WebEvent::SessionEnded);
                    return Ok(());
                }
            }
        }
    }
    Ok(())
}

// —— WebSocket ——

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    ws.on_upgrade(move |socket| ws_conn(socket, state))
}

async fn ws_conn(mut socket: WebSocket, state: Arc<AppState>) {
    // 同锁内取历史快照并订阅,保证回放与实时流之间无缝、无重复
    let (snapshot, mut rx) = {
        let h = state.sink.history.lock().unwrap();
        (h.clone(), state.sink.tx.subscribe())
    };
    for ev in snapshot {
        if send_event(&mut socket, &ev).await.is_err() {
            return;
        }
    }

    loop {
        tokio::select! {
            res = rx.recv() => match res {
                Ok(ev) => {
                    if send_event(&mut socket, &ev).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            },
            msg = socket.recv() => match msg {
                Some(Ok(Message::Text(t))) => {
                    let _ = state.input_tx.send(t.to_string());
                }
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(_)) => {}
                Some(Err(_)) => break,
            },
        }
    }
}

async fn send_event(socket: &mut WebSocket, ev: &WebEvent) -> std::result::Result<(), ()> {
    let json = serde_json::to_string(ev).map_err(|_| ())?;
    socket
        .send(Message::Text(json.into()))
        .await
        .map_err(|_| ())
}

// —— REST ——

async fn get_config() -> Response {
    match Config::load(None) {
        Ok(c) => Json(c).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct ConfigSet {
    scope: String,
    key: String,
    value: String,
}

async fn put_config(Json(req): Json<ConfigSet>) -> Response {
    let scope = if req.scope == "project" {
        Scope::Project
    } else {
        Scope::User
    };
    match config::set_value(scope, &req.key, &req.value) {
        Ok(_) => get_config().await,
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

#[derive(Serialize)]
struct Meta {
    model: String,
    thinking: bool,
    context_chars: usize,
}

async fn get_meta(State(state): State<Arc<AppState>>) -> Json<Meta> {
    let config = Config::load(None).unwrap_or_default();
    let context_chars = state
        .sink
        .history
        .lock()
        .unwrap()
        .iter()
        .map(event_len)
        .sum();
    Json(Meta {
        model: config.model.name,
        thinking: config.model.thinking,
        context_chars,
    })
}

/// 近似上下文字符数:累加各事件的文本长度。
fn event_len(ev: &WebEvent) -> usize {
    match ev {
        WebEvent::Reasoning { delta } | WebEvent::Content { delta } => delta.len(),
        WebEvent::CommandResult {
            narration,
            command,
            output,
            ..
        } => narration.len() + command.len() + output.len(),
        WebEvent::Prompt { message } => message.len(),
        WebEvent::UserInput { text } => text.len(),
        _ => 0,
    }
}

// —— 静态资源(SPA) ——

async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match Assets::get(path) {
        Some(content) => {
            let mime = content.metadata.mimetype();
            ([(header::CONTENT_TYPE, mime)], content.data.into_owned()).into_response()
        }
        // SPA 回退:未命中资源时返回 index.html,交由前端路由处理
        None => match Assets::get("index.html") {
            Some(content) => (
                [(header::CONTENT_TYPE, "text/html")],
                content.data.into_owned(),
            )
                .into_response(),
            None => (StatusCode::NOT_FOUND, "frontend not built").into_response(),
        },
    }
}
