<script>
  // 纯文本条目内容(user / prompt 等):保留换行。
  // 颜色与字号由外层 bodyClass(注册表)统一控制,本组件不写死样式。

  let { text = '' } = $props();
</script>

<span class="whitespace-pre-wrap">{text}</span>
