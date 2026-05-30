<script>
  import { onMount, tick } from 'svelte';
  import { createSession } from '$lib/session.svelte.js';
  import ConfigPanel from '$lib/ConfigPanel.svelte';

  const session = createSession();
  let meta = $state({ model: '', thinking: false, context_chars: 0 });
  let showConfig = $state(false);
  let draft = $state('');
  let scroller;

  onMount(() => {
    session.connect();
    refreshMeta();
    const id = setInterval(refreshMeta, 3000);
    return () => clearInterval(id);
  });

  async function refreshMeta() {
    try {
      meta = await (await fetch('/api/meta')).json();
    } catch {
      /* 服务未就绪时忽略 */
    }
  }

  // 新条目到达后滚动到底部
  $effect(() => {
    session.entries.length;
    tick().then(() => {
      if (scroller) scroller.scrollTop = scroller.scrollHeight;
    });
  });

  function submit() {
    const text = draft.trim();
    if (!text || !session.awaiting) return;
    session.send(text);
    draft = '';
  }

  function onKey(e) {
    // Enter 发送,Shift+Enter 换行
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }

  const placeholder = $derived(
    session.ended
      ? '会话已结束'
      : session.awaiting === 'prompt'
        ? '回复 Agent 的提问…'
        : session.awaiting === 'user'
          ? '输入你的请求…'
          : 'Agent 工作中…'
  );
</script>

<div class="flex h-screen flex-col bg-zinc-950 text-zinc-100">
  <!-- 顶部元信息条 -->
  <header
    class="flex items-center justify-between border-b border-zinc-800 bg-zinc-900/60 px-4 py-2 text-sm"
  >
    <div class="flex items-center gap-3">
      <span class="font-mono font-semibold text-amber-400">doit</span>
      <span
        class="inline-block h-2 w-2 rounded-full {session.connected
          ? 'bg-emerald-500'
          : 'bg-zinc-600'}"
        title={session.connected ? 'connected' : 'disconnected'}
      ></span>
    </div>
    <div class="flex items-center gap-4 text-zinc-400">
      <span class="font-mono">{meta.model}</span>
      <span title="reasoning">🧠 {meta.thinking ? 'on' : 'off'}</span>
      <span title="approx context chars">ctx ≈ {meta.context_chars.toLocaleString()}</span>
      <button
        class="rounded border border-zinc-700 px-2 py-0.5 hover:bg-zinc-800"
        onclick={() => (showConfig = !showConfig)}
      >
        {showConfig ? '对话' : '配置'}
      </button>
    </div>
  </header>

  {#if showConfig}
    <div class="flex-1 overflow-hidden p-4">
      <ConfigPanel />
    </div>
  {:else}
    <!-- 对话流 -->
    <div bind:this={scroller} class="flex-1 space-y-2 overflow-y-auto px-4 py-4">
      {#each session.entries as e, i (i)}
        {#if e.kind === 'reasoning'}
          <pre class="whitespace-pre-wrap font-mono text-sm italic text-zinc-500">{e.text}</pre>
        {:else if e.kind === 'content'}
          <div
            class="whitespace-pre-wrap rounded bg-amber-950/40 px-3 py-2 text-sm leading-relaxed text-zinc-100"
          >
            {e.text}
          </div>
        {:else if e.kind === 'user'}
          <div class="flex gap-2 border-l-2 border-amber-500 bg-zinc-900/40 px-3 py-2">
            <span class="font-mono text-amber-500">&gt;</span>
            <span class="whitespace-pre-wrap font-mono text-sm">{e.text}</span>
          </div>
        {:else if e.kind === 'prompt'}
          <div class="whitespace-pre-wrap rounded bg-amber-950/40 px-3 py-2 text-sm text-amber-100">
            {e.message}
          </div>
        {:else if e.kind === 'command'}
          <div class="overflow-hidden rounded border-l-2 border-blue-500 bg-zinc-900/70">
            {#if e.narration}
              <div class="px-3 pt-2 font-mono text-xs text-zinc-400"># {e.narration}</div>
            {/if}
            <button
              class="flex w-full items-center gap-2 px-3 py-2 text-left font-mono text-sm hover:bg-zinc-800/50"
              onclick={() => (e.collapsed = !e.collapsed)}
            >
              <span class="text-blue-400">$</span>
              <span class="flex-1 truncate">{e.command}</span>
              {#if e.exit_code !== 0}
                <span class="text-red-400">[{e.exit_code}]</span>
              {/if}
              <span class="text-zinc-600">{e.collapsed ? '▸' : '▾'}</span>
            </button>
            {#if !e.collapsed && e.output.trim()}
              <pre
                class="max-h-96 overflow-auto border-t border-zinc-800 bg-black/30 px-3 py-2 font-mono text-xs text-zinc-300">{e.output}</pre>
            {/if}
          </div>
        {/if}
      {/each}

      {#if session.ended}
        <div class="py-2 text-center text-xs text-zinc-600">— 会话已结束 —</div>
      {/if}
    </div>

    <!-- 输入区 -->
    <div class="border-t border-zinc-800 bg-zinc-900/60 p-3">
      <div
        class="flex items-end gap-2 rounded border {session.awaiting
          ? 'border-amber-500/60'
          : 'border-zinc-800'} bg-zinc-950 px-3 py-2"
      >
        <span class="pb-1 font-mono text-amber-500">&gt;</span>
        <textarea
          bind:value={draft}
          onkeydown={onKey}
          {placeholder}
          rows="1"
          disabled={session.ended}
          class="max-h-40 flex-1 resize-none bg-transparent font-mono text-sm text-zinc-100 placeholder:text-zinc-600 focus:outline-none disabled:opacity-50"
        ></textarea>
        <button
          onclick={submit}
          disabled={!session.awaiting || !draft.trim()}
          class="rounded bg-amber-600 px-3 py-1 text-sm font-medium text-white hover:bg-amber-500 disabled:cursor-not-allowed disabled:opacity-40"
        >
          发送
        </button>
      </div>
    </div>
  {/if}
</div>
