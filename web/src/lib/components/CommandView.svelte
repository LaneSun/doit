<script>
  import { onMount } from 'svelte';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import '@xterm/xterm/css/xterm.css';

  // fill=false(列表内联):高度自适应内容,最多 MAX_ROWS 行,超出则终端内滚动。
  // fill=true(详情侧栏):占满容器高宽,内容超出则终端内滚动。
  let { narration = '', command = '', output = '', exitCode = 0, fill = false } = $props();

  const MAX_ROWS = 24;
  const BG = '#09090b'; // zinc-950,与页面背景一致

  let container;
  let term = $state(null);
  let ready = $state(false); // 首次 draw 完成前隐藏,避免初始尺寸的闪变
  let lastWidth = 0;
  let rafId = 0;
  let writing = false; // xterm write 为异步队列,进行中不重入,避免两次 reset+write 叠加成重复输出
  let pending = false; // write 期间又有重绘请求,完成后补绘一次
  const fit = new FitAddon();

  // 合并同一帧内的多次重绘请求(挂载时 $effect 与 ResizeObserver 常同帧触发)
  function schedule() {
    if (!term) return;
    cancelAnimationFrame(rafId);
    rafId = requestAnimationFrame(draw);
  }

  onMount(() => {
    term = new Terminal({
      cursorBlink: false,
      disableStdin: true,
      fontSize: 12,
      fontFamily: "'Cascadia Code', 'Fira Code', 'JetBrains Mono', ui-monospace, monospace",
      theme: { background: BG, foreground: '#d4d4d8', cursor: BG, selectionBackground: '#3b3b3b' },
      rows: 1, // 从 1 行起,避免 xterm 默认 24 行在展开瞬间撑出超长终端再缩回的闪变
      cols: 80
    });
    term.loadAddon(fit);
    term.open(container);

    // 容器宽/高变化(展开、面板或窗口缩放)时按新尺寸重排
    const ro = new ResizeObserver(() => {
      const w = container?.clientWidth ?? 0;
      if (w && w !== lastWidth) {
        lastWidth = w;
        schedule();
      }
    });
    ro.observe(container);

    return () => {
      ro.disconnect();
      cancelAnimationFrame(rafId);
      const t = term;
      term = null; // 置空以阻止已排队的 rAF draw 在销毁后操作终端
      t?.dispose();
    };
  });

  // term 就绪或 props 变化(含详情面板切换复用)时重绘
  $effect(() => {
    void [narration, command, output, exitCode, fill];
    if (term) schedule();
  });

  function draw() {
    if (!term) return;
    // 串行化:上一次 write 未完成时不重入,记下 pending 待完成后补绘,
    // 保证任意时刻 xterm 写队列至多一份数据,从根上杜绝重复输出。
    if (writing) {
      pending = true;
      return;
    }
    writing = true;

    // 先按容器尺寸确定行列,使折行与最终列宽一致
    try {
      fit.fit();
    } catch {
      /* 容器尚未布局时忽略 */
    }
    term.reset();

    let data = '';
    if (narration) data += `\x1b[2m# ${narration}\x1b[0m\r\n`;
    if (command) {
      let line = `\x1b[1m$ ${command}\x1b[0m`;
      if (exitCode !== 0) line += `  \x1b[31m[${exitCode}]\x1b[0m`;
      data += `${line}\r\n`;
    }
    if (output) data += output.replace(/\r\n/g, '\n').replace(/\n/g, '\r\n');

    // write 是异步的:必须在回调里读取行数,否则 cursorY 尚未更新会把高度算成 1 行。
    term.write(data, () => {
      writing = false;
      if (!term) return;
      if (!fill) {
        const used = term.buffer.active.baseY + term.buffer.active.cursorY + 1;
        term.resize(term.cols, Math.max(1, Math.min(used, MAX_ROWS)));
      }
      ready = true;
      if (pending) {
        pending = false;
        schedule();
      }
    });
  }
</script>

<div bind:this={container} class="cmd-term" class:fill class:opacity-0={!ready}></div>

<style>
  .cmd-term {
    width: 100%;
    background: #09090b;
  }
  .cmd-term.fill {
    height: 100%;
  }
  /* 统一各层背景为 zinc-950,消除 xterm 默认黑底与内边距处的黑边 */
  .cmd-term :global(.xterm) {
    padding: 4px 8px;
    background-color: #09090b;
  }
  .cmd-term :global(.xterm .xterm-viewport),
  .cmd-term :global(.xterm .xterm-screen) {
    background-color: #09090b;
  }
  .cmd-term :global(.xterm-viewport) {
    scrollbar-width: thin;
    scrollbar-color: #3f3f46 transparent;
  }
</style>
