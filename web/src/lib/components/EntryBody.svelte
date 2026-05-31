<script>
  // 统一内容渲染器:按注册表 meta.body 动态渲染条目内容,列表展开态与详情侧栏共用。
  // bare 类型(终端)不加包裹边距;fill 透传给支持占满容器的内容组件(CommandView)。

  import { entryMeta } from '$lib/entries.js';

  let { entry, fill = false } = $props();

  const meta = $derived(entryMeta(entry.kind));
  const Body = $derived(meta.body);
  const bodyProps = $derived(meta.bodyProps(entry, { fill }));
</script>

{#if meta.bare}
  <Body {...bodyProps} />
{:else}
  <div class="py-1.5 pr-2 {meta.bodyClass ?? ''}">
    <Body {...bodyProps} />
  </div>
{/if}
