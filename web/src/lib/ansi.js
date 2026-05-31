// ANSI SGR (Select Graphic Rendition) → HTML converter.
// Handles basic SGR codes: colors, bold, dim, italic, underline, reset.
// Returns HTML string with inline-styled <span> elements.

const FG_COLORS = [
  '#c9d1d9', // 30: default (light gray)
  '#ef4444', // 31: red
  '#4ade80', // 32: green
  '#facc15', // 33: yellow
  '#60a5fa', // 34: blue
  '#c084fc', // 35: magenta
  '#22d3ee', // 36: cyan
  '#f8fafc'  // 37: white
];

const BG_COLORS = [
  'transparent',
  '#7f1d1d',
  '#14532d',
  '#713f12',
  '#1e3a5f',
  '#581c87',
  '#164e63',
  '#1e293b'
];

const BRIGHT_FG = [
  '#6b7280', '#f87171', '#86efac', '#fde047',
  '#93c5fd', '#d8b4fe', '#67e8f9', '#ffffff'
];

function styleString(styles) {
  const parts = [];
  if (styles.bold) parts.push('font-weight:bold');
  if (styles.dim) parts.push('opacity:0.6');
  if (styles.italic) parts.push('font-style:italic');
  if (styles.underline) parts.push('text-decoration:underline');
  if (styles.fg !== undefined) parts.push(`color:${pickFg(styles.fg)}`);
  if (styles.bg !== undefined) parts.push(`background:${pickBg(styles.bg)}`);
  return parts.join(';');
}

function pickFg(code) {
  if (code < 8) return FG_COLORS[code];
  if (code < 16) return BRIGHT_FG[code - 8];
  return FG_COLORS[0];
}

function pickBg(code) {
  if (code < 8) return BG_COLORS[code];
  if (code < 16) return BG_COLORS[code - 8]; // approximate bright bg
  return BG_COLORS[0];
}

export function ansiToHtml(text) {
  if (!text) return '';

  let out = '';
  let styles = {};
  let buf = '';
  let i = 0;

  const flush = () => {
    if (!buf) return;
    if (Object.keys(styles).length === 0) {
      out += escapeHtml(buf);
    } else {
      const s = styleString(styles);
      out += `<span style="${s}">${escapeHtml(buf)}</span>`;
    }
    buf = '';
  };

  while (i < text.length) {
    if (text[i] === '\x1b' && i + 1 < text.length && text[i + 1] === '[') {
      flush();
      let j = i + 2;
      while (j < text.length && text[j] !== 'm') j++;
      if (j >= text.length) break;
      const params = text.substring(i + 2, j).split(';').map(Number);
      for (const p of params) {
        if (p === 0) styles = {};
        else if (p === 1) styles.bold = true;
        else if (p === 2) styles.dim = true;
        else if (p === 3) styles.italic = true;
        else if (p === 4) styles.underline = true;
        else if (p >= 30 && p <= 37) styles.fg = p - 30;
        else if (p >= 40 && p <= 47) styles.bg = p - 40;
        else if (p >= 90 && p <= 97) styles.fg = p - 90 + 8;
        else if (p >= 100 && p <= 107) styles.bg = p - 100 + 8;
        else if (p === 39) delete styles.fg;
        else if (p === 49) delete styles.bg;
      }
      i = j + 1;
    } else {
      buf += text[i];
      i++;
    }
  }
  flush();
  return out;
}

function escapeHtml(s) {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}
