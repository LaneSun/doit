// 对话条目(entry)的呈现元数据集中表。
//
// 每个 WebEvent 被 session store 归一化为一条 entry,kind 决定其图标、左侧
// 竖条颜色、文字色以及是否可折叠。视图层只读这张表,不再各自硬编码。

import ChevronRight from 'lucide-svelte/icons/chevron-right';
import Sparkles from 'lucide-svelte/icons/sparkles';
import Brain from 'lucide-svelte/icons/brain';
import Terminal from 'lucide-svelte/icons/terminal';
import MessageSquare from 'lucide-svelte/icons/message-square';

/**
 * @typedef {'user' | 'content' | 'reasoning' | 'command' | 'prompt'} EntryKind
 * @typedef {Object} EntryMeta
 * @property {any} icon      lucide 图标组件
 * @property {string} bar    左侧竖条颜色(CSS 颜色值,含 rgba)
 * @property {string} text   图标/标题文字色(Tailwind 类)
 * @property {boolean} collapsible  是否可折叠(false 表示始终展开)
 */

/** @type {Record<EntryKind, EntryMeta>} */
export const ENTRY_META = {
  user: { icon: ChevronRight, bar: '#f59e0b', text: 'text-amber-500', collapsible: false },
  content: { icon: Sparkles, bar: '#d4d4d8', text: 'text-zinc-300', collapsible: false },
  reasoning: { icon: Brain, bar: 'rgba(113,113,122,0.4)', text: 'text-zinc-500', collapsible: true },
  command: { icon: Terminal, bar: 'rgba(96,165,250,0.4)', text: 'text-blue-400', collapsible: true },
  prompt: { icon: MessageSquare, bar: '#fde68a', text: 'text-amber-200', collapsible: false }
};

/** 取某 kind 的元数据,未知 kind 退回 content。 */
export function entryMeta(kind) {
  return ENTRY_META[kind] ?? ENTRY_META.content;
}
