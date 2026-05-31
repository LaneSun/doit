<script>
  import { ansiToHtml } from './ansi.js';

  let { text = '', maxHeight = '320px' } = $props();
</script>

<div
  class="overflow-auto bg-black font-mono text-xs leading-relaxed"
  style="max-height: {maxHeight}; padding: 8px 12px;"
>
  {#if text}
    {@html ansiToHtml(text)}
  {:else}
    <span class="text-zinc-600 text-xs">(empty)</span>
  {/if}
</div>
