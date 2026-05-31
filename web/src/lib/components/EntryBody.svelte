<script>
  // 一条 entry 的主体内容(不含图标/竖条/折叠外壳),按 kind 分发渲染。
  // 列表的展开态与右侧详情面板共用本组件,保证两处呈现一致(DRY)。

  import Markdown from './Markdown.svelte';
  import CommandView from './CommandView.svelte';

  let { entry } = $props();
</script>

{#if entry.kind === 'command'}
  <CommandView
    narration={entry.narration}
    command={entry.command}
    output={entry.output}
    exitCode={entry.exit_code}
  />
{:else if entry.kind === 'content'}
  <div class="py-1.5 pr-2 text-sm text-zinc-100">
    <Markdown text={entry.text} />
  </div>
{:else if entry.kind === 'reasoning'}
  <div class="py-1.5 pr-2 text-xs text-zinc-500">
    <Markdown text={entry.text} />
  </div>
{:else if entry.kind === 'user'}
  <div class="py-1.5 pr-2">
    <span class="text-sm whitespace-pre-wrap text-zinc-300">{entry.text}</span>
  </div>
{:else if entry.kind === 'prompt'}
  <div class="py-1.5 pr-2">
    <span class="text-sm whitespace-pre-wrap text-amber-200">{entry.message}</span>
  </div>
{/if}
