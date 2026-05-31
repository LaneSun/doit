<script>
  // 对话行外壳:左侧彩色竖条 + 居中列 + 图标槽,所有 kind 的历史条目共用,
  // 统一「竖条 / 缩进 / 图标对齐」的视觉语言,消除各处重复与魔数。
  // dense=true 用于单行(折叠预览),图标垂直居中;否则顶对齐(多行内容)。

  let { bar, icon: Icon, iconClass = '', dense = false, active = false, children } = $props();
</script>

<div class="relative {active ? 'bg-zinc-800/60' : ''}">
  <div class="absolute top-0 bottom-0 left-0 w-0.5" style="background:{bar}"></div>

  <div class="mx-auto max-w-[700px]">
    <div class="flex gap-2 pl-2 {dense ? 'h-9 items-center' : 'min-h-9 items-start'}">
      <div class="flex w-6 shrink-0 justify-center {dense ? 'h-full items-center' : 'pt-[11px]'}">
        <Icon size={14} class={iconClass} />
      </div>
      <div class="min-w-0 flex-1">
        {@render children()}
      </div>
    </div>
  </div>
</div>
