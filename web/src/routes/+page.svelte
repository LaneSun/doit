<script>
  import { onMount, tick } from 'svelte';
  import { createSession } from '$lib/session.svelte.js';
  import XtermOutput from '$lib/XtermOutput.svelte';
  import Markdown    from '$lib/Markdown.svelte';
  import { PaneGroup, Pane, PaneResizer } from 'paneforge';

  // ── icons ──
  import ChevronRight   from 'lucide-svelte/icons/chevron-right';
  import TerminalIcon   from 'lucide-svelte/icons/terminal';
  import Brain          from 'lucide-svelte/icons/brain';
  import Sparkles       from 'lucide-svelte/icons/sparkles';
  import MessageSquare  from 'lucide-svelte/icons/message-square';
  import PanelRight     from 'lucide-svelte/icons/panel-right';
  import PanelRightClose from 'lucide-svelte/icons/panel-right-close';
  import Settings       from 'lucide-svelte/icons/settings';

  const session = createSession();

  // ── meta ──
  let meta = $state({ model: '', thinking: false, context_chars: 0 });

  // ── viewport ──
  let wide = $state(false);
  let rightCollapsed = $state(false);
  const canUseRightPanel = $derived(wide && !rightCollapsed);

  // ── input ──
  let draft = $state('');
  let scroller;

  // ── lifecycle ──
  onMount(() => {
    session.connect();
    refreshMeta();
    const mq = window.matchMedia('(min-width: 1200px)');
    wide = mq.matches;
    mq.onchange = (e) => (wide = e.matches);
    const id = setInterval(refreshMeta, 3000);
    return () => clearInterval(id);
  });

  async function refreshMeta() {
    try {
      meta = await (await fetch('/api/meta')).json();
    } catch { /* server not ready */ }
  }

  // ── auto-scroll ──
  $effect(() => {
    session.entries.length;
    tick().then(() => {
      if (!scroller) return;
      const atBottom = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 80;
      if (atBottom) scroller.scrollTop = scroller.scrollHeight;
    });
  });

  // ── entry click ──
  function onEntryClick(i) {
    if (canUseRightPanel) {
      session.setActive(session.activeIndex === i ? -1 : i);
    } else {
      session.toggleExpanded(i);
    }
  }

  // ── input ──
  function submit() {
    const text = draft.trim();
    if (!text || !session.awaiting) return;
    session.send(text);
    draft = '';
  }

  function onKey(e) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
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

  const activeEntry = $derived(
    session.activeIndex >= 0 ? session.entries[session.activeIndex] : null
  );

  // ── per-type config ──
  function entryCfg(kind) {
    switch (kind) {
      case 'user':      return { bar: '#f59e0b', icon: ChevronRight,  textColor: 'text-amber-500', alwaysExpanded: true,  clickable: false };
      case 'content':   return { bar: '#d4d4d8', icon: Sparkles,      textColor: 'text-zinc-300',  alwaysExpanded: true,  clickable: true  };
      case 'reasoning': return { bar: 'rgba(113,113,122,0.4)', icon: Brain,         textColor: 'text-zinc-500',  alwaysExpanded: false, clickable: true  };
      case 'command':   return { bar: 'rgba(96,165,250,0.4)', icon: TerminalIcon,  textColor: 'text-blue-400',  alwaysExpanded: false, clickable: true  };
      case 'prompt':    return { bar: '#fde68a', icon: MessageSquare, textColor: 'text-amber-200', alwaysExpanded: true,  clickable: false };
      default:          return { bar: '#3f3f46', icon: Sparkles,      textColor: 'text-zinc-500',  alwaysExpanded: true,  clickable: false };
    }
  }
</script>

<!-- ═══════════════ left-panel snippet ═══════════════ -->
{#snippet leftPanelContent()}
  <div class="flex flex-col flex-1 min-w-0">
    <div bind:this={scroller} class="flex-1 overflow-y-auto">

      {#each session.entries as e, i (i)}
        {@const cfg = entryCfg(e.kind)}
        {@const expanded = cfg.alwaysExpanded || (!canUseRightPanel && e.expanded)}
        {@const isActive = canUseRightPanel && session.activeIndex === i}

        <!-- full-width wrapper: color bar flush to viewport/pane edge -->
        <div class="relative {isActive ? 'bg-zinc-800/60' : ''}">
          <div class="absolute left-0 top-0 bottom-0 w-0.5" style="background:{cfg.bar}"></div>

          <div class="max-w-[700px] mx-auto">
            <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
            <div
              class="{cfg.clickable ? 'cursor-pointer' : ''}"
              onclick={cfg.clickable ? () => onEntryClick(i) : undefined}
              role={cfg.clickable ? 'button' : undefined}
              tabindex={cfg.clickable ? 0 : -1}
            >
              <!-- ═════════ user · always expanded ═════════ -->
              {#if e.kind === 'user'}
                <div class="flex items-start gap-2 pl-2 min-h-9">
                  <div class="shrink-0 w-6 flex justify-center pt-[11px]">
                    <ChevronRight size={14} class="text-amber-500" />
                  </div>
                  <div class="flex-1 min-w-0 py-1.5 pr-2">
                    <span class="whitespace-pre-wrap text-sm text-zinc-300">{e.text}</span>
                  </div>
                </div>

              <!-- ═════════ content · always expanded ═════════ -->
              {:else if e.kind === 'content'}
                <div class="flex items-start gap-2 pl-2 min-h-9">
                  <div class="shrink-0 w-6 flex justify-center pt-[11px]">
                    <Sparkles size={14} class="text-zinc-300" />
                  </div>
                  <div class="flex-1 min-w-0 py-1.5 pr-2">
                    <Markdown text={e.text} />
                  </div>
                </div>

              <!-- ═════════ reasoning ═════════ -->
              {:else if e.kind === 'reasoning'}
                {#if expanded}
                  <div class="flex items-start gap-2 pl-2 min-h-9">
                    <div class="shrink-0 w-6 flex justify-center pt-[11px]">
                      <Brain size={14} class="text-zinc-500" />
                    </div>
                    <div class="flex-1 min-w-0 py-1.5 pr-2 text-xs text-zinc-500">
                      <Markdown text={e.text} />
                    </div>
                  </div>
                {:else}
                  <div class="flex items-center gap-2 h-9 pl-2">
                    <div class="shrink-0 w-6 flex items-center justify-center h-full">
                      <Brain size={14} class="text-zinc-500" />
                    </div>
                    <span class="flex-1 text-xs text-zinc-600 truncate">{e.text.slice(0, 100)}</span>
                  </div>
                {/if}

              <!-- ═════════ command ═════════ -->
              {:else if e.kind === 'command'}
                {#if expanded}
                  <div class="flex items-start gap-2 pl-2 min-h-9">
                    <div class="shrink-0 w-6 flex justify-center pt-[11px]">
                      <TerminalIcon size={14} class="text-blue-400" />
                    </div>
                    <div class="flex-1 min-w-0">
                      <div role="presentation" onpointerdown={(ev) => ev.stopPropagation()} onclick={(ev) => ev.stopPropagation()} onkeydown={(ev) => ev.stopPropagation()}>
                        <XtermOutput text={e.output} narration={e.narration} command={e.command} exitCode={e.exit_code} />
                      </div>
                    </div>
                  </div>
                {:else}
                  <div class="flex items-center gap-2 h-9 pl-2">
                    <div class="shrink-0 w-6 flex items-center justify-center h-full">
                      <TerminalIcon size={14} class="text-blue-400" />
                    </div>
                    {#if e.narration}
                      <span class="flex-1 text-xs text-zinc-600 truncate">{e.narration}</span>
                    {:else}
                      <span class="flex-1 text-xs text-zinc-600 truncate">{e.command}</span>
                    {/if}
                    {#if e.exit_code !== 0}
                      <span class="shrink-0 text-xs text-red-400">[{e.exit_code}]</span>
                    {/if}
                  </div>
                {/if}

              <!-- ═════════ prompt · always expanded ═════════ -->
              {:else if e.kind === 'prompt'}
                <div class="flex items-start gap-2 pl-2 min-h-9">
                  <div class="shrink-0 w-6 flex justify-center pt-[11px]">
                    <MessageSquare size={14} class="text-amber-200" />
                  </div>
                  <div class="flex-1 min-w-0 py-1.5 pr-2">
                    <span class="whitespace-pre-wrap text-sm text-amber-200">{e.message}</span>
                  </div>
                </div>
              {/if}
            </div>
          </div>
        </div>

        <!-- separator (full width) -->
        {#if i < session.entries.length - 1 || session.awaiting === null}
          <div class="border-t border-zinc-900"></div>
        {/if}
      {/each}

      <!-- ═════════ INPUT AREA (console-style, contenteditable) ═════════ -->
      <div class="border-t border-zinc-900"></div>

      <div class="relative">
        <div class="absolute left-0 top-0 bottom-0 w-0.5" style="background:#f59e0b"></div>

        <div class="max-w-[700px] mx-auto">
          <div class="flex items-start gap-2 pl-2 min-h-9">
            <div class="shrink-0 w-6 flex justify-center pt-[11px]">
              <ChevronRight size={14} class="text-amber-500" />
            </div>
            <div class="flex-1 min-w-0 py-1.5 pr-2">
              <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
              <div
                contenteditable="true"
                role="textbox"
                aria-multiline="true"
                tabindex="0"
                data-placeholder={placeholder}
                bind:textContent={draft}
                onkeydown={session.ended ? (e) => e.preventDefault() : onKey}
                onbeforeinput={session.ended ? (e) => e.preventDefault() : undefined}
                class="console-input"
                class:ended={session.ended}
              ></div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
{/snippet}

<div class="flex flex-col h-screen bg-zinc-950 text-zinc-100">

  <!-- ═══════════════ MAIN CONTENT ═══════════════ -->
  {#if canUseRightPanel}
    <PaneGroup direction="horizontal" autoSaveId="doit-panels" class="flex-1 overflow-hidden min-h-0">
      <Pane defaultSize={60} minSize={30}>
        {@render leftPanelContent()}
      </Pane>

      <PaneResizer class="w-px bg-zinc-800 hover:bg-amber-500 transition-colors" />

      <!-- ═════════ RIGHT PANEL (pure content view, no icons) ═════════ -->
      <Pane defaultSize={40} minSize={15}>
        <div class="flex flex-col h-full border-l border-zinc-800">
          {#if activeEntry}
            <div class="flex-1 overflow-y-auto">
              {#if activeEntry.kind === 'command'}
                <XtermOutput
                  text={activeEntry.output}
                  narration={activeEntry.narration}
                  command={activeEntry.command}
                  exitCode={activeEntry.exit_code}
                />

              {:else if activeEntry.kind === 'reasoning'}
                <div class="py-1.5 pl-2 pr-2 text-xs text-zinc-400">
                  <Markdown text={activeEntry.text} />
                </div>

              {:else if activeEntry.kind === 'content'}
                <div class="py-1.5 pl-2 pr-2">
                  <Markdown text={activeEntry.text} />
                </div>

              {:else if activeEntry.kind === 'user'}
                <div class="py-1.5 pl-2 pr-2">
                  <span class="whitespace-pre-wrap text-sm text-zinc-300">{activeEntry.text}</span>
                </div>

              {:else if activeEntry.kind === 'prompt'}
                <div class="py-1.5 pl-2 pr-2">
                  <span class="whitespace-pre-wrap text-sm text-amber-200">{activeEntry.message}</span>
                </div>

              {:else}
                <div class="py-1.5 pl-2 pr-2 text-xs text-zinc-600">
                  No details
                </div>
              {/if}
            </div>
          {:else}
            <div class="flex items-center justify-center h-full">
              <span class="text-xs text-zinc-600">Select an entry to view details</span>
            </div>
          {/if}
        </div>
      </Pane>
    </PaneGroup>
  {:else}
    <div class="flex flex-1 overflow-hidden min-h-0">
      {@render leftPanelContent()}
    </div>
  {/if}

  <!-- ═══════════════ BOTTOM NAV BAR ═══════════════ -->
  <div class="shrink-0 border-t border-zinc-800 bg-zinc-900">
    <div class="flex items-stretch justify-between h-8">
      <div class="flex items-center gap-3 ml-3 text-xs text-zinc-500">
        <span class="text-zinc-400">{meta.model || '—'}</span>
        <span class="flex items-center gap-1">
          <Brain size={12} />
          <span>{meta.thinking ? 'on' : 'off'}</span>
        </span>
        <span>ctx {meta.context_chars.toLocaleString()}</span>
      </div>

      <div class="flex items-stretch">
        {#if wide}
          <button
            onclick={() => (rightCollapsed = !rightCollapsed)}
            class="flex items-center justify-center w-8 hover:bg-zinc-800 text-zinc-500 hover:text-zinc-300"
            title={rightCollapsed ? 'show right panel' : 'hide right panel'}
          >
            {#if rightCollapsed}
              <PanelRight size={14} />
            {:else}
              <PanelRightClose size={14} />
            {/if}
          </button>
        {/if}
        <button
          class="flex items-center justify-center w-8 hover:bg-zinc-800 text-zinc-500 hover:text-zinc-300"
          title="settings"
        >
          <Settings size={14} />
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .console-input {
    outline: none !important;
    box-shadow: none !important;
    border: none;
    background: transparent;
    color: #f4f4f5;
    font-size: 0.875rem;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
    min-height: 1.5em;
    caret-color: #f59e0b;
  }
  .console-input.ended {
    opacity: 0.5;
    cursor: default;
  }
  .console-input:empty::before {
    content: attr(data-placeholder);
    color: #52525b;
    pointer-events: none;
  }
</style>
