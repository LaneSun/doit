<script>
  // 底部状态栏:左侧元信息(连接状态/模型/推理/上下文),右侧面板与设置控制。

  import Brain from 'lucide-svelte/icons/brain';
  import PanelRight from 'lucide-svelte/icons/panel-right';
  import PanelRightClose from 'lucide-svelte/icons/panel-right-close';
  import Settings from 'lucide-svelte/icons/settings';

  let {
    meta = { model: '', thinking: false, context_chars: 0 },
    connected = false,
    wide = false,
    rightCollapsed = false,
    onTogglePanel,
    onOpenSettings
  } = $props();
</script>

<div class="flex h-8 shrink-0 items-stretch justify-between border-t border-zinc-800 bg-zinc-900">
  <div class="ml-3 flex items-center gap-3 text-xs text-zinc-500">
    <span
      class="inline-block h-1.5 w-1.5 {connected ? 'bg-emerald-500' : 'bg-zinc-600'}"
      title={connected ? 'connected' : 'disconnected'}
    ></span>
    <span class="text-zinc-400">{meta.model || '—'}</span>
    <span class="flex items-center gap-1">
      <Brain size={12} />
      <span>{meta.thinking ? 'on' : 'off'}</span>
    </span>
    <span>ctx {meta.context_chars.toLocaleString()}</span>
  </div>

  <div class="flex items-stretch">
    {#if wide}
      <button
        onclick={onTogglePanel}
        class="flex w-8 items-center justify-center text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300"
        title={rightCollapsed ? 'show detail panel' : 'hide detail panel'}
      >
        {#if rightCollapsed}
          <PanelRight size={14} />
        {:else}
          <PanelRightClose size={14} />
        {/if}
      </button>
    {/if}
    <button
      onclick={onOpenSettings}
      class="flex w-8 items-center justify-center text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300"
      title="settings"
    >
      <Settings size={14} />
    </button>
  </div>
</div>
