<script>
  import { onMount } from 'svelte';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import '@xterm/xterm/css/xterm.css';

  let { text = '', narration = '', command = '', exitCode = 0 } = $props();
  let container;

  onMount(() => {
    const ZINC_950 = '#09090b';

    const term = new Terminal({
      cursorBlink: false,
      disableStdin: true,
      fontSize: 12,
      fontFamily: "'Cascadia Code', 'Fira Code', 'JetBrains Mono', ui-monospace, monospace",
      theme: {
        background: ZINC_950,
        foreground: '#d4d4d8',
        cursor: ZINC_950,
        selectionBackground: '#3b3b3b',
      },
      rows: 20,   // generous start — no scroll during write
      cols: 80,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(container);

    // ── write content ──
    if (narration) {
      term.write('\x1b[2m# ' + narration + '\x1b[0m\r\n');
    }
    if (command) {
      let line = '\x1b[1m$ ' + command + '\x1b[0m';
      if (exitCode !== 0) {
        line += '  \x1b[31m[' + exitCode + ']\x1b[0m';
      }
      term.write(line + '\r\n');
    }
    if (text) {
      term.write(text);
    }

    // ── size rows to actual content ──
    const totalLines = term.buffer.active.baseY + term.buffer.active.cursorY + 1;
    const rows = Math.max(1, Math.min(totalLines, 20));
    term.resize(term.cols, rows);

    // ── lock height before fit, so fit() only adjusts columns ──
    requestAnimationFrame(() => {
      const h = term.element.clientHeight;
      if (h > 0) container.style.height = h + 'px';
      try { fitAddon.fit(); } catch { /* ignore */ }
      container.style.height = '';
    });

    return () => term.dispose();
  });
</script>

<div bind:this={container} class="xterm-container"></div>

<style>
  .xterm-container {
    width: 100%;
  }
  .xterm-container :global(.xterm) {
    padding: 4px 8px;
  }
  .xterm-container :global(.xterm-viewport) {
    scrollbar-width: thin;
    scrollbar-color: #3f3f46 transparent;
  }
</style>
