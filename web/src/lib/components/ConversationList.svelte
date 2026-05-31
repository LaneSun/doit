<script>
  // 对话流:逐条渲染 entry + 末尾输入框,并在贴近底部时自动滚动。
  // 折叠/可点击/选中等呈现策略集中在此,ConversationEntry 只负责画出来。
  // detail=是否处于「详情面板可见」模式(双栏且侧栏未隐藏);否则就地展开。

  import { tick } from 'svelte';
  import { entryMeta } from '$lib/entries.js';
  import ConversationEntry from './ConversationEntry.svelte';
  import Composer from './Composer.svelte';

  let { session, detail = false } = $props();

  let scroller;

  // 仅当用户已贴近底部时才跟随滚动,避免打断回看。
  // 依赖同时覆盖条目数与末条流式文本长度:reasoning/content 增量是就地追加
  // (数组长度不变),若只依赖 length,流式输出时将不会滚动。
  $effect(() => {
    const list = session.entries;
    const last = list[list.length - 1];
    void (list.length + (last?.text?.length ?? 0) + (last?.output?.length ?? 0));
    tick().then(() => {
      if (!scroller) return;
      const gap = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight;
      if (gap < 80) scroller.scrollTop = scroller.scrollHeight;
    });
  });

  // 详情模式:可折叠项折叠成单行,详情在右面板;否则(单栏或侧栏隐藏)就地展开
  const isExpanded = (e) => !entryMeta(e.kind).collapsible || (!detail && e.expanded);
  // 详情模式:可进详情的条目可点击;否则仅可折叠项可点击展开
  const isClickable = (e) => (detail ? entryMeta(e.kind).detail : entryMeta(e.kind).collapsible);

  function onEntryClick(i) {
    if (detail) session.toggleActive(i);
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
      active={detail && session.activeIndex === i}
      onClick={() => onEntryClick(i)}
    />
    <div class="border-t border-zinc-900"></div>
  {/each}

  <Composer {placeholder} disabled={inputDisabled} onSubmit={(t) => session.send(t)} />
</div>
