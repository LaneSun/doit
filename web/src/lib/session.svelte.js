// WebSocket 会话状态:把下行 WebEvent 流转换为可渲染的对话条目。
export function createSession() {
  let entries = $state([]);
  let awaiting = $state(null); // null | 'user' | 'prompt'
  let ended = $state(false);
  let connected = $state(false);
  let streamKind = null; // 当前正在流式的块类型('reasoning' | 'content')
  let ws = null;

  function appendStream(kind, delta) {
    const last = entries[entries.length - 1];
    if (streamKind === kind && last && last.kind === kind) {
      last.text += delta;
    } else {
      entries.push({ kind, text: delta });
      streamKind = kind;
    }
  }

  function handle(ev) {
    switch (ev.type) {
      case 'reasoning':
        appendStream('reasoning', ev.delta);
        break;
      case 'content':
        appendStream('content', ev.delta);
        break;
      case 'stream_end':
        streamKind = null;
        break;
      case 'command_result':
        streamKind = null;
        entries.push({
          kind: 'command',
          narration: ev.narration,
          command: ev.command,
          output: ev.output,
          exit_code: ev.exit_code,
          is_exit: ev.is_exit,
          collapsed: true
        });
        break;
      case 'prompt':
        streamKind = null;
        entries.push({ kind: 'prompt', message: ev.message });
        awaiting = 'prompt';
        break;
      case 'await_user':
        awaiting = 'user';
        break;
      case 'user_input':
        entries.push({ kind: 'user', text: ev.text });
        awaiting = null;
        break;
      case 'session_ended':
        ended = true;
        awaiting = null;
        break;
    }
  }

  function connect() {
    const proto = location.protocol === 'https:' ? 'wss' : 'ws';
    ws = new WebSocket(`${proto}://${location.host}/ws`);
    ws.onopen = () => (connected = true);
    ws.onclose = () => (connected = false);
    ws.onmessage = (e) => {
      try {
        handle(JSON.parse(e.data));
      } catch {
        /* 忽略无法解析的帧 */
      }
    };
  }

  function send(text) {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(text);
      awaiting = null;
    }
  }

  return {
    get entries() {
      return entries;
    },
    get awaiting() {
      return awaiting;
    },
    get ended() {
      return ended;
    },
    get connected() {
      return connected;
    },
    connect,
    send
  };
}
