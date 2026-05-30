# 后续任务规划

> 本文件记录已确认但尚未完成的较大任务。每个任务遵循「提方向 → 设计讨论 → 实现细节 → 编码」四步流程，逐项推进。

---

## 任务 1：分级配置系统（TOML）

### 目标

用 TOML 实现完整的多层级配置，覆盖：

- **API 配置**：base_url、api_key（支持 `${ENV}` 引用）等
- **模型配置**：model、temperature、max_tokens、thinking 开关等
- **交互式显示配置**：命令输出显隐、推理显隐、narration 显隐等
- **Prompt 配置**：覆盖内置系统提示（交互/任务两种模式）

### 优先级链（高 → 低，见 DESIGN.md）

1. CLI 参数（`--model`、`--no-truncate` 等）
2. 环境变量（`DOIT_API__BASE_URL`，双下划线表嵌套）
3. `--config <path>` 指定文件
4. `./doit.toml`（项目级，当前工作目录）
5. `~/.config/doit/config.toml`（用户级）
6. 内置默认值

### 当前硬编码点（需迁移到配置）

- `src/agent/mod.rs`：`DEFAULT_MODEL`、`Agent::from_env()`（api_key/base_url/model）
- `src/commands/exec/mod.rs`：`DEFAULT_CHARS=2000`、`DEFAULT_LINES=50`
- `src/agent/mod.rs`：`InteractiveView`/`StreamRender`（reasoning/content/narration/命令输出渲染）
- 系统提示模板（`template` 命令）

### 已定方案

- crate：`figment`（TOML+Env 分层）+ `directories`（XDG）+ `toml_edit`（无损写回）+ `toml`
- schema：`[api] [model] [output] [display] [prompt] [locale]`
- `[prompt]`：`append_*`（追加，保留命令注册表）+ `system_*`（完全覆盖逃生舱）
- 新增 `doit config` 子命令：`path` / `list` / `get <key>` / `set <key> <value> [--project]`

### 状态：已完成（待用户审计）

---

## 任务 2：Web 后端（SvelteKit + Tailwind）

### 目标

交互模式启动后输出一个 localhost 服务地址（`--host` 可开放到局域网），提供 UI 版 Agent 界面：

- 对话与命令条目，默认折叠；运行中的命令展开且支持交互式输入
- 元信息面板：上下文用量、模型、推理强度等
- 配置界面：修改项目级与用户级配置（复用任务 1 的配置系统）

### 已定方向

- 启动形态：`doit web [--host --port]` 独立子命令
- 第一版范围：只读对话/命令展示 + 流式输出 + 用户输入 + 配置界面；运行中命令的 PTY↔xterm 交互后置为独立阶段
- 前端：`web/` 子目录，SvelteKit + Tailwind，用 **bun** 构建，产物 `rust-embed` 嵌入二进制
- 传输：HTTP REST（配置/元信息/历史快照）+ WebSocket（下行事件流、上行用户输入）
- Rust 侧：`axum` + `rust-embed`

### 架构要点

- Agent 循环跑在专用 std::thread（内置 current-thread tokio runtime 驱动 async 后端）；axum 在主 runtime
- Agent → Web：`tokio::broadcast<WebEvent>` + 共享事件历史缓冲（供新连接回放）
- Web → Agent：`std::mpsc<String>`（专用线程阻塞 recv）
- 复用 `llm_turn`；`TurnView` 增加默认方法 `handle_prompt`（web 接管 doit prompt，终端回退到子进程）
- 新增 `WebView`（实现 `TurnView`）把流式增量与命令结果转 JSON 事件；非交互 `ShellSession`（仅捕获输出）

### 分阶段

1. 事件模型 `WebEvent` + `WebView`（TurnView）+ `handle_prompt` 钩子 — 已完成
2. `web` 子命令 + axum 骨架（HTTP+WS+静态资源+URL 输出+`--host/--port`）+ Agent 线程桥接 — 已完成
3. SvelteKit + Tailwind 前端：对话/命令折叠列表、流式渲染、元信息、配置界面 — 已完成
4. （后置）运行中命令 PTY ↔ xterm.js 交互桥接 — 待做

### 状态：第一版已完成（待用户审计）；运行中命令的 xterm 交互留待后置阶段
