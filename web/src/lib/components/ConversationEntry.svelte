<script>
  // 列表中的一条对话条目:左侧彩色竖条 + 居中内容列(图标 + 主体)。
  // 展开态渲染完整 EntryBody;折叠态(仅 reasoning/command)渲染单行预览。
  // 点击行为由父层通过 onclick 决定(宽屏:选入详情面板;窄屏:内联展开)。

  import { entryMeta } from '$lib/entries.js';
  import EntryBody from './EntryBody.svelte';

  let { entry, expanded = false, clickable = false, active = false, onclick } = $props();

  const meta = $derived(entryMeta(entry.kind));
  const Icon = $derived(meta.icon);
  const preview = $derived(
    entry.kind === 'command' ? entry.narration || entry.command : (entry.text ?? '').slice(0, 100)
  );

  function onkeydown(e) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onclick?.();
    }
  }
  // 命令展开态:拦截内部点击冒泡,避免选中输出文本时误触整行折叠
  function stop(e) {
    e.stopPropagation();
  }
</script>

<div class="relative {active ? 'bg-zinc-800/60' : ''}">
  <div class="absolute top-0 bottom-0 left-0 w-0.5" style="background:{meta.bar}"></div>

  <div class="mx-auto max-w-[700px]">
    <div
      class={clickable ? 'cursor-pointer' : ''}
      onclick={clickable ? () => onclick?.() : undefined}
      onkeydown={clickable ? onkeydown : undefined}
      role={clickable ? 'button' : undefined}
      tabindex={clickable ? 0 : -1}
    >
      {#if expanded}
        <div class="flex min-h-9 items-start gap-2 pl-2">
          <div class="flex w-6 shrink-0 justify-center pt-[11px]">
            <Icon size={14} class={meta.text} />
          </div>
          {#if entry.kind === 'command'}
            <div class="min-w-0 flex-1" role="presentation" onclick={stop} onkeydown={stop}>
              <EntryBody {entry} />
            </div>
          {:else}
            <div class="min-w-0 flex-1">
              <EntryBody {entry} />
            </div>
          {/if}
        </div>
      {:else}
        <div class="flex h-9 items-center gap-2 pl-2">
          <div class="flex h-full w-6 shrink-0 items-center justify-center">
            <Icon size={14} class={meta.text} />
          </div>
          <span class="flex-1 truncate text-xs text-zinc-600">{preview}</span>
          {#if entry.kind === 'command' && entry.exit_code !== 0}
            <span class="shrink-0 pr-2 text-xs text-red-400">[{entry.exit_code}]</span>
          {/if}
        </div>
      {/if}
    </div>
  </div>
</div>
