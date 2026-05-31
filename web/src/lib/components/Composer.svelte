<script>
  // 控制台风格输入:左侧 `>` 图标 + 自适应高度的多行输入。
  // Enter 发送、Shift+Enter 换行;禁用态(会话结束/非等待)下吞掉输入。

  import ChevronRight from 'lucide-svelte/icons/chevron-right';

  let { placeholder = '', disabled = false, onsubmit } = $props();

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
    onsubmit?.(text);
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

<div class="mx-auto max-w-[700px]">
  <div class="flex min-h-9 items-start gap-2 pl-2">
    <div class="flex w-6 shrink-0 justify-center pt-[11px]">
      <ChevronRight size={14} class="text-amber-500" />
    </div>
    <div class="min-w-0 flex-1 py-1.5 pr-2">
      <textarea
        bind:this={textarea}
        bind:value={draft}
        {placeholder}
        {disabled}
        rows="1"
        oninput={autosize}
        {onkeydown}
        class="console-input w-full resize-none bg-transparent text-sm text-zinc-100 placeholder:text-zinc-600 focus:outline-none disabled:opacity-50"
      ></textarea>
    </div>
  </div>
</div>

<style>
  .console-input {
    line-height: 1.5;
    caret-color: #f59e0b;
    overflow: hidden;
    max-height: 40vh;
  }
</style>
