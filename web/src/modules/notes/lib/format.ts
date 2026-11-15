export type NoteFormat = 'bold' | 'italic' | 'list' | 'link'

export interface FormatResult {
  text: string
  start: number
  end: number
}

function wrap(text: string, start: number, end: number, mark: string): FormatResult {
  const selected = text.slice(start, end)
  if (selected.startsWith(mark) && selected.endsWith(mark) && selected.length >= mark.length * 2) {
    const inner = selected.slice(mark.length, selected.length - mark.length)
    return { text: text.slice(0, start) + inner + text.slice(end), start, end: start + inner.length }
  }
  const body = selected === '' ? '' : selected
  return {
    text: text.slice(0, start) + mark + body + mark + text.slice(end),
    start: start + mark.length,
    end: start + mark.length + body.length,
  }
}

function list(text: string, start: number, end: number): FormatResult {
  const lineStart = text.lastIndexOf('\n', start - 1) + 1
  const lineEndIndex = text.indexOf('\n', end)
  const lineEnd = lineEndIndex === -1 ? text.length : lineEndIndex
  const lines = text.slice(lineStart, lineEnd).split('\n')
  const allListed = lines.every((line) => /^\s*[-*+]\s/.test(line))
  const next = lines.map((line) => (allListed ? line.replace(/^(\s*)[-*+]\s/, '$1') : `- ${line}`)).join('\n')
  return { text: text.slice(0, lineStart) + next + text.slice(lineEnd), start: lineStart, end: lineStart + next.length }
}

function link(text: string, start: number, end: number): FormatResult {
  const selected = text.slice(start, end)
  const label = selected === '' ? 'texto' : selected
  const url = 'https://'
  const inserted = `[${label}](${url})`
  const urlStart = start + label.length + 3
  return {
    text: text.slice(0, start) + inserted + text.slice(end),
    start: urlStart,
    end: urlStart + url.length,
  }
}

export function applyFormat(text: string, start: number, end: number, kind: NoteFormat): FormatResult {
  switch (kind) {
    case 'bold':
      return wrap(text, start, end, '**')
    case 'italic':
      return wrap(text, start, end, '*')
    case 'list':
      return list(text, start, end)
    case 'link':
      return link(text, start, end)
  }
}
