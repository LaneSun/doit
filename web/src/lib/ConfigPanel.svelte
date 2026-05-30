<script>
  let config = $state(null);
  let scope = $state('user');
  let status = $state('');

  async function load() {
    try {
      config = await (await fetch('/api/config')).json();
    } catch (e) {
      status = 'load error';
    }
  }
  load();

  async function save(key, value) {
    status = 'saving…';
    const res = await fetch('/api/config', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ scope, key, value: String(value) })
    });
    if (res.ok) {
      config = await res.json();
      status = `saved ${key} → ${scope}`;
    } else {
      status = 'error: ' + (await res.text());
    }
  }

  // 仅展示标量叶子(字符串/数字/布尔),按 section 分组
  function leaves(section, obj) {
    if (!obj) return [];
    return Object.entries(obj)
      .filter(([, v]) => ['string', 'number', 'boolean'].includes(typeof v))
      .map(([k, v]) => ({ key: `${section}.${k}`, name: k, value: v, type: typeof v }));
  }
</script>

<div class="flex h-full flex-col">
  <div class="mb-3 flex items-center gap-3">
    <span class="text-sm text-zinc-400">写入层级</span>
    <select
      bind:value={scope}
      class="rounded border border-zinc-700 bg-zinc-800 px-2 py-1 text-sm text-zinc-100"
    >
      <option value="user">用户级 (~/.config/doit)</option>
      <option value="project">项目级 (./doit.toml)</option>
    </select>
    {#if status}
      <span class="text-xs text-zinc-500">{status}</span>
    {/if}
  </div>

  {#if !config}
    <p class="text-sm text-zinc-500">加载中…</p>
  {:else}
    <div class="flex-1 space-y-5 overflow-y-auto pr-1">
      {#each ['api', 'model', 'output', 'display'] as section (section)}
        <div>
          <h3 class="mb-2 font-mono text-xs uppercase tracking-wider text-amber-400/80">
            [{section}]
          </h3>
          <div class="space-y-2">
            {#each leaves(section, config[section]) as f (f.key)}
              <label class="flex items-center gap-3 text-sm">
                <span class="w-40 shrink-0 font-mono text-zinc-400">{f.name}</span>
                {#if f.type === 'boolean'}
                  <input
                    type="checkbox"
                    checked={f.value}
                    onchange={(e) => save(f.key, e.currentTarget.checked)}
                    class="h-4 w-4 accent-amber-500"
                  />
                {:else}
                  <input
                    type={f.type === 'number' ? 'number' : 'text'}
                    step="any"
                    value={f.value}
                    onchange={(e) => save(f.key, e.currentTarget.value)}
                    class="flex-1 rounded border border-zinc-700 bg-zinc-800 px-2 py-1 font-mono text-zinc-100 focus:border-amber-500 focus:outline-none"
                  />
                {/if}
              </label>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
