<script>
  // 控制台风格输入:竖条 + > 图标沿用 user 强调色,父容器零内距、最小高度与条目行一致(min-h-9),
  // 文字垂直居中、随内容向下增长。Enter 发送、Shift+Enter 换行;禁用态吞掉输入。

  import ChevronRight from 'lucide-svelte/icons/chevron-right';

  let { placeholder = '', disabled = false, onSubmit } = $props();

  let draft = $state('');
  let textarea;

  function autosize() {
    if (!textarea) return;
    textarea.style.height = 'auto';
    textarea.style.height = `${textarea.scrollHeight}px`;
  }

  function submit() {
    const text = draft.trim();
    if (!text || disabled) return;
    onSubmit?.(text);
    draft = '';
    queueMicrotask(autosize);
  }

  function onkeydown(e) {
    if (disabled) {
      e.preventDefault();
      return;
    }
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }
</script>

<div class="relative">
  <div class="absolute inset-y-0 left-0 w-0.5" style="background:var(--color-accent)"></div>
  <div class="mx-auto flex min-h-9 max-w-[700px] items-center gap-2 pr-2 pl-2">
    <div class="flex w-6 shrink-0 items-center justify-center">
      <ChevronRight size={14} class="text-accent" />
    </div>
    <textarea
      bind:this={textarea}
      bind:value={draft}
      {placeholder}
      {disabled}
      rows="1"
      oninput={autosize}
      {onkeydown}
      class="console-input min-w-0 flex-1 resize-none bg-transparent text-sm text-zinc-100 placeholder:text-zinc-600 disabled:opacity-50"
    ></textarea>
  </div>
</div>

<style>
  .console-input {
    padding: 0;
    line-height: 1.5;
    caret-color: var(--color-accent);
    overflow: hidden;
    max-height: 40vh;
  }
  /* 压过 app.css 中无 @layer 的全局 *:focus-visible 描边(unlayered 优先级高于 utilities) */
  .console-input:focus,
  .console-input:focus-visible {
    outline: none;
  }
</style>
