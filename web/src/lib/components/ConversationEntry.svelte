<script>
  // 历史条目的统一容器:所有 kind 共用同一外壳(竖条 + 图标 + 折叠/展开)。
  // 呈现完全由注册表驱动(图标/色/预览/角标),容器本身不含任何 kind 分支。
  // 可点击行用原生 <button> 承载(键盘可达、无 a11y 误报),否则裸渲染。

  import { entryMeta } from '$lib/entries.js';
  import EntryShell from './EntryShell.svelte';
  import EntryBody from './EntryBody.svelte';

  let { entry, expanded = false, clickable = false, active = false, onClick } = $props();

  const meta = $derived(entryMeta(entry.kind));
  const preview = $derived(meta.preview ? meta.preview(entry) : '');
  const badge = $derived(meta.badge ? meta.badge(entry) : null);

  // bare(终端)展开态:拦截内部点击冒泡,避免选中输出时误触整行折叠/选中
  function stop(e) {
    e.stopPropagation();
  }
</script>

{#snippet row()}
  <EntryShell bar={meta.bar} icon={meta.icon} iconClass={meta.text} dense={!expanded} {active}>
    {#if expanded}
      {#if meta.bare}
        <div role="presentation" onclick={stop} onkeydown={stop}>
          <EntryBody {entry} />
        </div>
      {:else}
        <EntryBody {entry} />
      {/if}
    {:else}
      <div class="flex items-center gap-2 pr-2">
        <span class="flex-1 truncate text-xs text-zinc-600">{preview}</span>
        {#if badge}
          <span class="shrink-0 text-xs text-red-400">{badge}</span>
        {/if}
      </div>
    {/if}
  </EntryShell>
{/snippet}

{#if clickable}
  <button type="button" class="block w-full cursor-pointer text-left" onclick={() => onClick?.()}>
    {@render row()}
  </button>
{:else}
  {@render row()}
{/if}
