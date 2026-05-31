// WebSocket session state: converts downstream WebEvent stream into renderable
// conversation entries, plus right-panel selection state.

export function createSession() {
  let entries = $state([]);
  let awaiting = $state(null);    // null | 'user' | 'prompt'
  let ended = $state(false);
  let connected = $state(false);
  let streamKind = null;          // current streaming block type ('reasoning' | 'content')
  let activeIndex = $state(-1);   // selected entry index (for right panel)

  // ── append streaming delta ──
  function appendStream(kind, delta) {
    const last = entries[entries.length - 1];
    if (streamKind === kind && last && last.kind === kind) {
      last.text += delta;
    } else {
      entries.push({ kind, text: delta });
      streamKind = kind;
    }
  }

  // ── process a single WebEvent ──
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
      case 'command_result': {
        streamKind = null;
        const entry = {
          kind: 'command',
          narration: ev.narration,
          command: ev.command,
          output: ev.output,
          exit_code: ev.exit_code,
          is_exit: ev.is_exit,
          expanded: false
        };
        entries.push(entry);
        // auto-select if right panel is active
        break;
      }
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

  // ── WebSocket connect ──
  let ws = null;

  function connect() {
    const proto = location.protocol === 'https:' ? 'wss' : 'ws';
    ws = new WebSocket(`${proto}://${location.host}/ws`);
    ws.onopen = () => (connected = true);
    ws.onclose = () => (connected = false);
    ws.onmessage = (e) => {
      try {
        handle(JSON.parse(e.data));
      } catch {
        /* ignore unparseable frames */
      }
    };
  }

  function send(text) {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(text);
      awaiting = null;
    }
  }

  // ── right panel selection ──
  function setActive(index) {
    activeIndex = index;
  }

  function clearActive() {
    activeIndex = -1;
  }

  // ── toggle inline expansion (for when no right panel) ──
  function toggleExpanded(index) {
    const e = entries[index];
    if (e) {
      e.expanded = !e.expanded;
    }
  }

  return {
    get entries() { return entries; },
    get awaiting() { return awaiting; },
    get ended() { return ended; },
    get connected() { return connected; },
    get activeIndex() { return activeIndex; },
    connect,
    send,
    setActive,
    clearActive,
    toggleExpanded
  };
}
