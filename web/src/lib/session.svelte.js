// WebSocket 会话状态:把后端下行的 WebEvent 流归一化为可渲染的对话条目,
// 并维护输入等待态与右面板选中态。返回的对象通过 getter 暴露响应式只读视图,
// 变更只能经由内部 handler / 动作函数发生,避免视图层直接改状态。

/**
 * @typedef {Object} Entry
 * @property {string} kind       'user'|'content'|'reasoning'|'command'|'prompt'
 * @property {string} [text]     user/content/reasoning 的文本
 * @property {string} [message]  prompt 的提示语
 * @property {string} [narration] command 的概述
 * @property {string} [command]  command 的命令行
 * @property {string} [output]   command 的输出(可含 ANSI)
 * @property {number} [exit_code]
 * @property {boolean} [is_exit]
 * @property {boolean} [expanded] 窄屏内联展开态
 */

/** 可流式累积的块类型(同类相邻增量并入同一条 entry)。 */
const STREAM_KINDS = new Set(['reasoning', 'content']);

export function createSession() {
  let entries = $state(/** @type {Entry[]} */ ([]));
  let awaiting = $state(/** @type {null | 'user' | 'prompt'} */ (null));
  let ended = $state(false);
  let connected = $state(false);
  let activeIndex = $state(-1); // 右面板选中项,-1 表示未选

  let streamKind = null; // 当前正在累积的流式块类型
  /** @type {WebSocket | null} */
  let ws = null;

  /** 把流式增量并入末尾同类条目,否则新建一条。 */
  function appendStream(kind, delta) {
    const last = entries[entries.length - 1];
    if (streamKind === kind && last?.kind === kind) {
      last.text += delta;
    } else {
      entries.push({ kind, text: delta });
      streamKind = kind;
    }
  }

  /** 处理单条下行 WebEvent。 */
  function handle(ev) {
    switch (ev.type) {
      case 'reasoning':
      case 'content':
        appendStream(ev.type, ev.delta);
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
          expanded: false
        });
        break;
      case 'prompt':
        streamKind = null;
        entries.push({ kind: 'prompt', message: ev.message });
        awaiting = 'prompt';
        break;
      case 'await_user':
        streamKind = null;
        awaiting = 'user';
        break;
      case 'user_input':
        streamKind = null;
        entries.push({ kind: 'user', text: ev.text });
        awaiting = null;
        break;
      case 'session_ended':
        ended = true;
        awaiting = null;
        break;
    }
  }

  let reconnectTimer = null;

  // 重连成功后清空本地状态,交由历史回放重建,避免条目重复
  function reset() {
    entries = [];
    awaiting = null;
    ended = false;
    activeIndex = -1;
    streamKind = null;
  }

  // 断线自动重连,依赖后端历史回放无缝恢复
  function connect() {
    clearTimeout(reconnectTimer);
    const proto = location.protocol === 'https:' ? 'wss' : 'ws';
    ws = new WebSocket(`${proto}://${location.host}/ws`);
    ws.onopen = () => {
      connected = true;
      reset();
    };
    ws.onmessage = (e) => {
      try {
        handle(JSON.parse(e.data));
      } catch {
        /* 忽略无法解析的帧 */
      }
    };
    ws.onclose = () => {
      connected = false;
      if (!ended) reconnectTimer = setTimeout(connect, 1000); // 会话已结束则不再重连
    };
  }

  /** 发送一条用户输入(顶层回合或 doit prompt 回复共用)。 */
  function send(text) {
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(text);
      awaiting = null;
    }
  }

  /** 选中右面板项;传入当前项则取消选中(toggle)。 */
  function toggleActive(index) {
    activeIndex = activeIndex === index ? -1 : index;
  }

  /** 窄屏下内联展开/折叠某条目。 */
  function toggleExpanded(index) {
    const e = entries[index];
    if (e) e.expanded = !e.expanded;
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
    get activeIndex() {
      return activeIndex;
    },
    get activeEntry() {
      return activeIndex >= 0 ? entries[activeIndex] : null;
    },
    connect,
    send,
    toggleActive,
    toggleExpanded
  };
}
