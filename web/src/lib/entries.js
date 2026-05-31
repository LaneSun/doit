// 对话条目(entry)类型注册表。
//
// 每个 WebEvent 被 session store 归一化为一条 entry,kind 决定其全部呈现规格:
// 图标、左侧竖条色、文字色、是否可折叠,以及「内容如何渲染」(body 组件 + props 映射)、
// 折叠态预览文本与角标。视图层(ConversationEntry / EntryBody)只查这张表,不写 kind 分支。
// 新增类型(含未来经 doit 环境 socket 自定义输出的)只需在此注册一项 + 提供内容组件。

import ChevronRight from 'lucide-svelte/icons/chevron-right';
import Sparkles from 'lucide-svelte/icons/sparkles';
import Brain from 'lucide-svelte/icons/brain';
import Terminal from 'lucide-svelte/icons/terminal';
import MessageSquare from 'lucide-svelte/icons/message-square';

import Markdown from './components/Markdown.svelte';
import CommandView from './components/CommandView.svelte';
import TextBody from './components/TextBody.svelte';

/**
 * @typedef {'user'|'content'|'reasoning'|'command'|'prompt'} EntryKind
 * @typedef {Object} EntryMeta
 * @property {any} icon            lucide 图标组件
 * @property {string} bar          左侧竖条颜色(CSS 值;强调色统一引用 --color-accent)
 * @property {string} text         图标/标题文字色(Tailwind 类)
 * @property {boolean} collapsible 窄屏是否可就地折叠/展开(false=始终展开)
 * @property {boolean} detail      宽屏是否可点击进入右侧详情面板
 * @property {any} body            内容渲染组件
 * @property {(entry:any, ctx:{fill:boolean})=>object} bodyProps  从 entry 映射内容组件 props
 * @property {string} [bodyClass]  内容包裹层类(字号/颜色由此控制)
 * @property {boolean} [bare]      true=内容自带边距、不加包裹(如终端)
 * @property {(entry:any)=>string} [preview]        折叠态单行预览文本
 * @property {(entry:any)=>(string|null)} [badge]   折叠态右侧角标(如退出码)
 */

/** @type {Record<EntryKind, EntryMeta>} */
export const ENTRY_META = {
  user: {
    icon: ChevronRight,
    bar: 'var(--color-accent)',
    text: 'text-accent',
    collapsible: false,
    detail: false,
    body: TextBody,
    bodyProps: (e) => ({ text: e.text }),
    bodyClass: 'text-sm text-zinc-300'
  },
  content: {
    icon: Sparkles,
    bar: '#d4d4d8',
    text: 'text-zinc-300',
    collapsible: false,
    detail: true,
    body: Markdown,
    bodyProps: (e) => ({ text: e.text }),
    bodyClass: 'text-sm text-zinc-100'
  },
  reasoning: {
    icon: Brain,
    bar: 'rgba(113,113,122,0.4)',
    text: 'text-zinc-500',
    collapsible: true,
    detail: true,
    body: Markdown,
    bodyProps: (e) => ({ text: e.text }),
    bodyClass: 'text-xs text-zinc-500',
    preview: (e) => (e.text ?? '').slice(0, 100)
  },
  command: {
    icon: Terminal,
    bar: 'rgba(96,165,250,0.4)',
    text: 'text-blue-400',
    collapsible: true,
    detail: true,
    body: CommandView,
    bare: true,
    bodyProps: (e, ctx) => ({
      narration: e.narration,
      command: e.command,
      output: e.output,
      exitCode: e.exit_code,
      fill: ctx.fill
    }),
    preview: (e) => e.narration || e.command,
    badge: (e) => (e.exit_code !== 0 ? `[${e.exit_code}]` : null)
  },
  prompt: {
    icon: MessageSquare,
    bar: '#fde68a',
    text: 'text-amber-200',
    collapsible: false,
    detail: false,
    body: TextBody,
    bodyProps: (e) => ({ text: e.message }),
    bodyClass: 'text-sm text-amber-200'
  }
};

/** 取某 kind 的元数据,未知 kind 退回 content。 */
export function entryMeta(kind) {
  return ENTRY_META[kind] ?? ENTRY_META.content;
}
