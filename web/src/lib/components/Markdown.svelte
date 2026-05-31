<script module>
  import { marked } from 'marked';

  // 模块级一次性配置(无副作用、所有实例共享):换行敏感 + GFM。
  // 内容来自本机 LLM 输出,作为单用户本地工具不额外引入 HTML 消毒。
  marked.setOptions({ breaks: true, gfm: true });

  function render(text) {
    try {
      return marked.parse(text ?? '');
    } catch {
      return (text ?? '').replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    }
  }
</script>

<script>
  let { text = '' } = $props();
  const html = $derived(render(text));
</script>

<div class="md">{@html html}</div>

<style>
  .md {
    line-height: 1.6;
    word-break: break-word;
  }
  .md :global(p) {
    margin: 0 0 0.5em 0;
  }
  .md :global(p:last-child) {
    margin-bottom: 0;
  }
  .md :global(h1),
  .md :global(h2),
  .md :global(h3),
  .md :global(h4) {
    margin: 0.75em 0 0.25em 0;
    font-weight: 600;
    color: #e4e4e7;
  }
  .md :global(h1) {
    font-size: 1.1em;
  }
  .md :global(h2) {
    font-size: 1.05em;
  }
  .md :global(h3) {
    font-size: 1em;
  }
  .md :global(ul),
  .md :global(ol) {
    margin: 0 0 0.5em 0;
    padding-left: 1.5em;
  }
  .md :global(li) {
    margin: 0.15em 0;
  }
  .md :global(code) {
    font-family: 'Cascadia Code', 'Fira Code', 'JetBrains Mono', ui-monospace, monospace;
    font-size: 0.85em;
    background: #1c1c1e;
    padding: 1px 4px;
  }
  .md :global(pre) {
    margin: 0.5em 0;
    padding: 8px 12px;
    background: #0d0d0e;
    overflow-x: auto;
    font-family: 'Cascadia Code', 'Fira Code', 'JetBrains Mono', ui-monospace, monospace;
    font-size: 0.8em;
    line-height: 1.4;
  }
  .md :global(pre code) {
    background: none;
    padding: 0;
    font-size: inherit;
  }
  .md :global(blockquote) {
    margin: 0.5em 0;
    padding-left: 12px;
    border-left: 2px solid #3f3f46;
    color: #a1a1aa;
  }
  .md :global(hr) {
    border: none;
    border-top: 1px solid #27272a;
    margin: 0.75em 0;
  }
  .md :global(a) {
    color: #60a5fa;
    text-decoration: underline;
  }
  .md :global(table) {
    border-collapse: collapse;
    margin: 0.5em 0;
    font-size: 0.85em;
  }
  .md :global(th),
  .md :global(td) {
    border: 1px solid #3f3f46;
    padding: 4px 8px;
    text-align: left;
  }
  .md :global(th) {
    background: #1c1c1e;
    font-weight: 600;
  }
  .md :global(strong) {
    font-weight: 600;
    color: #e4e4e7;
  }
</style>
