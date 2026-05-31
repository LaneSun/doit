<script>
  // 设置覆盖层:承载配置面板。点击遮罩或 Esc 关闭。

  import ConfigPanel from './ConfigPanel.svelte';
  import X from 'lucide-svelte/icons/x';

  let { onclose } = $props();

  function onkeydown(e) {
    if (e.key === 'Escape') onclose?.();
  }
</script>

<svelte:window {onkeydown} />

<div
  class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
  onclick={onclose}
  role="presentation"
>
  <div
    class="flex max-h-[80vh] w-full max-w-2xl flex-col border border-zinc-700 bg-zinc-950"
    onclick={(e) => e.stopPropagation()}
    role="dialog"
    aria-modal="true"
    aria-label="Settings"
  >
    <div class="flex items-center justify-between border-b border-zinc-800 px-4 py-2">
      <h2 class="text-sm font-medium text-zinc-300">Settings</h2>
      <button onclick={onclose} class="text-zinc-500 hover:text-zinc-200" title="close">
        <X size={16} />
      </button>
    </div>
    <div class="min-h-0 flex-1 overflow-hidden p-4">
      <ConfigPanel />
    </div>
  </div>
</div>
