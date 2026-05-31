<script>
  import { marked } from 'marked';
  import { onMount } from 'svelte';

  let { text = '' } = $props();
  let html = $state('');
  let container;

  // Configure marked for agent content (no raw HTML passthrough)
  onMount(() => {
    marked.setOptions({
      breaks: true,       // single \n → <br>
      gfm: true,
    });
  });

  // Re-render on every text change (handles streaming)
  $effect(() => {
    const t = text;
    try {
      html = marked.parse(t) ;
    } catch {
      html = escapeHtml(t);
    }
  });

  function escapeHtml(s) {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }
</script>

<div bind:this={container} class="md-content">{@html html}</div>

<style>
  .md-content {
    line-height: 1.6;
    word-break: break-word;
  }
  .md-content :global(p) {
    margin: 0 0 0.5em 0;
  }
  .md-content :global(p:last-child) {
    margin-bottom: 0;
  }
  .md-content :global(h1),
  .md-content :global(h2),
  .md-content :global(h3),
  .md-content :global(h4) {
    margin: 0.75em 0 0.25em 0;
    font-weight: 600;
    color: #e4e4e7;
  }
  .md-content :global(h1) { font-size: 1.1em; }
  .md-content :global(h2) { font-size: 1.05em; }
  .md-content :global(h3) { font-size: 1em; }
  .md-content :global(ul),
  .md-content :global(ol) {
    margin: 0 0 0.5em 0;
    padding-left: 1.5em;
  }
  .md-content :global(li) {
    margin: 0.15em 0;
  }
  .md-content :global(code) {
    font-family: 'Cascadia Code', 'Fira Code', 'JetBrains Mono', ui-monospace, monospace;
    font-size: 0.85em;
    background: #1c1c1e;
    padding: 1px 4px;
    border-radius: 2px;
  }
  .md-content :global(pre) {
    margin: 0.5em 0;
    padding: 8px 12px;
    background: #0d0d0e;
    overflow-x: auto;
    font-family: 'Cascadia Code', 'Fira Code', 'JetBrains Mono', ui-monospace, monospace;
    font-size: 0.8em;
    line-height: 1.4;
  }
  .md-content :global(pre code) {
    background: none;
    padding: 0;
    font-size: inherit;
    border-radius: 0;
  }
  .md-content :global(blockquote) {
    margin: 0.5em 0;
    padding-left: 12px;
    border-left: 2px solid #3f3f46;
    color: #a1a1aa;
  }
  .md-content :global(hr) {
    border: none;
    border-top: 1px solid #27272a;
    margin: 0.75em 0;
  }
  .md-content :global(a) {
    color: #60a5fa;
    text-decoration: underline;
  }
  .md-content :global(table) {
    border-collapse: collapse;
    margin: 0.5em 0;
    font-size: 0.85em;
  }
  .md-content :global(th),
  .md-content :global(td) {
    border: 1px solid #3f3f46;
    padding: 4px 8px;
    text-align: left;
  }
  .md-content :global(th) {
    background: #1c1c1e;
    font-weight: 600;
  }
  .md-content :global(strong) {
    font-weight: 600;
    color: #e4e4e7;
  }
</style>
