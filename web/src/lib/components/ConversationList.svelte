<script>
  // 对话流:逐条渲染 entry + 末尾输入框,并在贴近底部时自动滚动。
  // 折叠/可点击/选中等呈现策略集中在此,ConversationEntry 只负责画出来。

  import { tick } from 'svelte';
  import ConversationEntry from './ConversationEntry.svelte';
  import Composer from './Composer.svelte';

  let { session, wide = false } = $props();

  const COLLAPSIBLE = new Set(['reasoning', 'command']);
  const CLICKABLE = new Set(['content', 'reasoning', 'command']);

  let scroller;

  // entry 增减后,仅当用户已贴近底部时才跟随滚动,避免打断回看
  $effect(() => {
    void session.entries.length;
    tick().then(() => {
      if (!scroller) return;
      const gap = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight;
      if (gap < 80) scroller.scrollTop = scroller.scrollHeight;
    });
  });

  // 宽屏:可折叠项折叠成单行,详情在右面板;窄屏:就地展开
  const isExpanded = (e) => !COLLAPSIBLE.has(e.kind) || (!wide && e.expanded);
  // 宽屏:正文类均可点击进入详情;窄屏:仅可折叠项可点击展开
  const isClickable = (e) => (wide ? CLICKABLE.has(e.kind) : COLLAPSIBLE.has(e.kind));

  function onEntryClick(i) {
    if (wide) session.toggleActive(i);
    else session.toggleExpanded(i);
  }

  const placeholder = $derived(
    session.ended
      ? 'Session ended'
      : session.awaiting === 'prompt'
        ? 'Reply to agent…'
        : session.awaiting === 'user'
          ? 'Type your request…'
          : 'Agent working…'
  );
  const inputDisabled = $derived(session.ended || session.awaiting === null);
</script>

<div bind:this={scroller} class="flex-1 overflow-y-auto">
  {#each session.entries as entry, i (i)}
    <ConversationEntry
      {entry}
      expanded={isExpanded(entry)}
      clickable={isClickable(entry)}
      active={wide && session.activeIndex === i}
      onclick={() => onEntryClick(i)}
    />
    <div class="border-t border-zinc-900"></div>
  {/each}

  <Composer {placeholder} disabled={inputDisabled} onsubmit={(t) => session.send(t)} />
</div>
