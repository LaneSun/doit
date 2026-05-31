<script>
  // 页面编排:连接会话、轮询元信息、按视口宽度在单栏/双栏间切换,
  // 并挂载底部状态栏与设置覆盖层。具体渲染下放到各组件。

  import { onMount } from 'svelte';
  import { PaneGroup, Pane, PaneResizer } from 'paneforge';
  import { createSession } from '$lib/session.svelte.js';
  import ConversationList from '$lib/components/ConversationList.svelte';
  import EntryBody from '$lib/components/EntryBody.svelte';
  import StatusBar from '$lib/components/StatusBar.svelte';
  import SettingsModal from '$lib/components/SettingsModal.svelte';

  const session = createSession();

  let meta = $state({ model: '', thinking: false, context_chars: 0 });
  let wide = $state(false);
  let rightCollapsed = $state(false);
  let showSettings = $state(false);

  const showDetail = $derived(wide && !rightCollapsed);

  onMount(() => {
    session.connect();
    refreshMeta();
    const mq = window.matchMedia('(min-width: 1200px)');
    wide = mq.matches;
    mq.onchange = (e) => (wide = e.matches);
    const timer = setInterval(refreshMeta, 3000);
    return () => clearInterval(timer);
  });

  async function refreshMeta() {
    try {
      meta = await (await fetch('/api/meta')).json();
    } catch {
      /* 服务尚未就绪,下次轮询重试 */
    }
  }
</script>

<div class="flex h-screen flex-col bg-zinc-950 text-zinc-100">
  {#if showDetail}
    <PaneGroup direction="horizontal" autoSaveId="doit-panels" class="min-h-0 flex-1 overflow-hidden">
      <Pane defaultSize={60} minSize={30}>
        <div class="flex h-full flex-col">
          <ConversationList {session} {wide} />
        </div>
      </Pane>

      <PaneResizer class="w-px bg-zinc-800 transition-colors hover:bg-amber-500" />

      <Pane defaultSize={40} minSize={15}>
        <div class="flex h-full flex-col border-l border-zinc-800">
          {#if session.activeEntry}
            <div
              class="flex-1 overflow-y-auto {session.activeEntry.kind === 'command' ? '' : 'pl-2'}"
            >
              {#key session.activeIndex}
                <EntryBody entry={session.activeEntry} />
              {/key}
            </div>
          {:else}
            <div class="flex h-full items-center justify-center">
              <span class="text-xs text-zinc-600">Select an entry to view details</span>
            </div>
          {/if}
        </div>
      </Pane>
    </PaneGroup>
  {:else}
    <div class="flex min-h-0 flex-1 flex-col overflow-hidden">
      <ConversationList {session} {wide} />
    </div>
  {/if}

  <StatusBar
    {meta}
    connected={session.connected}
    {wide}
    {rightCollapsed}
    onTogglePanel={() => (rightCollapsed = !rightCollapsed)}
    onOpenSettings={() => (showSettings = true)}
  />
</div>

{#if showSettings}
  <SettingsModal onclose={() => (showSettings = false)} />
{/if}
