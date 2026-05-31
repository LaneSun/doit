<script>
  // 配置面板:读取生效配置(GET /api/config),按 section 展示标量项,
  // 修改即写回所选层级(PUT /api/config)。仅展示标量叶子,隐藏 api_key 等敏感项。

  const SECTIONS = ['api', 'model', 'output', 'display'];
  const HIDDEN = new Set(['api.api_key']); // 不在 UI 暴露明文密钥

  let config = $state(null);
  let scope = $state('user');
  let status = $state('');

  async function load() {
    try {
      config = await (await fetch('/api/config')).json();
    } catch {
      status = 'failed to load config';
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
      status = `error: ${await res.text()}`;
    }
  }

  // 取某 section 下的标量字段(string/number/boolean),并标注类型供渲染对应控件
  function fields(section) {
    const obj = config?.[section];
    if (!obj) return [];
    return Object.entries(obj)
      .filter(([, v]) => ['string', 'number', 'boolean'].includes(typeof v))
      .map(([name, value]) => ({ key: `${section}.${name}`, name, value, type: typeof value }))
      .filter((f) => !HIDDEN.has(f.key));
  }
</script>

<div class="flex h-full flex-col gap-4">
  <div class="flex items-center gap-3 text-sm">
    <span class="text-zinc-400">Write to</span>
    <select
      bind:value={scope}
      class="border border-zinc-700 bg-zinc-800 px-2 py-1 text-zinc-100 focus:border-amber-500 focus:outline-none"
    >
      <option value="user">user (~/.config/doit)</option>
      <option value="project">project (./doit.toml)</option>
    </select>
    {#if status}
      <span class="text-xs text-zinc-500">{status}</span>
    {/if}
  </div>

  {#if !config}
    <p class="text-sm text-zinc-500">Loading…</p>
  {:else}
    <div class="flex-1 space-y-5 overflow-y-auto pr-1">
      {#each SECTIONS as section (section)}
        <section>
          <h3 class="mb-2 font-mono text-xs uppercase tracking-wider text-amber-400/80">
            [{section}]
          </h3>
          <div class="space-y-2">
            {#each fields(section) as f (f.key)}
              <label class="flex items-center gap-3 text-sm">
                <span class="w-44 shrink-0 font-mono text-zinc-400">{f.name}</span>
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
                    class="flex-1 border border-zinc-700 bg-zinc-800 px-2 py-1 font-mono text-zinc-100 focus:border-amber-500 focus:outline-none"
                  />
                {/if}
              </label>
            {/each}
          </div>
        </section>
      {/each}
    </div>
  {/if}
</div>
