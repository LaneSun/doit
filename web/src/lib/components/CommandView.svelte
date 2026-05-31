<script>
  import { onMount } from 'svelte';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import '@xterm/xterm/css/xterm.css';

  let { narration = '', command = '', output = '', exitCode = 0 } = $props();

  const MAX_ROWS = 24; // 超出则在终端内部滚动,避免单条命令占满视口
  const BG = '#09090b'; // zinc-950

  let container;
  let term = $state(null);
  let lastWidth = 0;
  const fit = new FitAddon();

  onMount(() => {
    term = new Terminal({
      cursorBlink: false,
      disableStdin: true,
      fontSize: 12,
      fontFamily: "'Cascadia Code', 'Fira Code', 'JetBrains Mono', ui-monospace, monospace",
      theme: { background: BG, foreground: '#d4d4d8', cursor: BG, selectionBackground: '#3b3b3b' },
      rows: MAX_ROWS,
      cols: 80
    });
    term.loadAddon(fit);
    term.open(container);

    // 容器宽度变化(面板/窗口缩放)时按新列宽重排
    const ro = new ResizeObserver(() => {
      const w = container?.clientWidth ?? 0;
      if (w && w !== lastWidth) {
        lastWidth = w;
        requestAnimationFrame(draw);
      }
    });
    ro.observe(container);

    return () => {
      ro.disconnect();
      const t = term;
      term = null; // 置空以阻止已排队的 rAF draw 在销毁后操作终端
      t?.dispose();
    };
  });

  // term 就绪或 props 变化(如详情面板切换复用本组件)时重绘
  $effect(() => {
    // 显式读取依赖
    void [narration, command, output, exitCode];
    if (term) requestAnimationFrame(draw);
  });

  function draw() {
    if (!term) return;
    // 先按容器宽度确定列数,再写入,使折行与最终列宽一致
    try {
      fit.fit();
    } catch {
      /* 容器尚未布局时忽略 */
    }
    term.reset();

    if (narration) term.write(`\x1b[2m# ${narration}\x1b[0m\r\n`);
    if (command) {
      let line = `\x1b[1m$ ${command}\x1b[0m`;
      if (exitCode !== 0) line += `  \x1b[31m[${exitCode}]\x1b[0m`;
      term.write(`${line}\r\n`);
    }
    if (output) term.write(output.replace(/\r\n/g, '\n').replace(/\n/g, '\r\n'));

    // 仅按实际内容行数调整高度(列数不变,不触发回流)
    const used = term.buffer.active.baseY + term.buffer.active.cursorY + 1;
    term.resize(term.cols, Math.max(1, Math.min(used, MAX_ROWS)));
  }
</script>

<div bind:this={container} class="cmd-term"></div>

<style>
  .cmd-term {
    width: 100%;
  }
  .cmd-term :global(.xterm) {
    padding: 4px 8px;
  }
  .cmd-term :global(.xterm-viewport) {
    scrollbar-width: thin;
    scrollbar-color: #3f3f46 transparent;
  }
</style>
