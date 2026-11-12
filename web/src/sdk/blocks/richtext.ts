import { micromark } from 'micromark'
import type { JSONContent } from '@tiptap/react'

const UNSUPPORTED = /^\s*\|.*\|\s*$|^\s*\[\^|<\/?[a-z][^>]*>|^\s*~~~|\n\s*\n\s*\n/im

export function markdownToHtml(markdown: string): string {
  return micromark(markdown)
}

export function needsSourceMode(markdown: string): boolean {
  return UNSUPPORTED.test(markdown)
}

function escapeText(text: string): string {
  return text.replace(/([\\`*_[\]<])/g, '\\$1')
}

function escapeLineStart(line: string): string {
  return line.replace(/^(\s*)([#>+-]|\d+[.)])(?=\s|$)/, '$1\\$2')
}

function inline(nodes: JSONContent[] | undefined): string {
  let result = ''
  for (const node of nodes ?? []) {
    if (node.type === 'hardBreak') {
      result += '  \n'
      continue
    }
    let text = node.text ?? ''
    const marks = node.marks ?? []
    const isCode = marks.some((mark) => mark.type === 'code')
    const leading = text.match(/^\s*/)?.[0] ?? ''
    const trailing = text.trim() === '' ? '' : (text.match(/\s*$/)?.[0] ?? '')
    text = text.trim() === '' ? text : text.slice(leading.length, text.length - trailing.length)
    if (text.trim() === '') {
      result += text
      continue
    }
    let rendered = isCode ? `\`${text.replace(/`/g, '')}\`` : escapeText(text)
    for (const mark of marks) {
      if (mark.type === 'bold') rendered = `**${rendered}**`
      else if (mark.type === 'italic') rendered = `*${rendered}*`
      else if (mark.type === 'link') rendered = `[${rendered}](${String(mark.attrs?.href ?? '').replace(/[()\s]/g, (character) => ({ '(': '%28', ')': '%29' })[character] ?? '%20')})`
    }
    result += `${leading}${rendered}${trailing}`
  }
  return result
}

function indent(text: string, prefix: string, firstPrefix = prefix): string {
  return text
    .split('\n')
    .map((line, index) => (line === '' ? '' : `${index === 0 ? firstPrefix : prefix}${line}`))
    .join('\n')
}

function listItem(node: JSONContent, marker: string): string {
  const [first, ...rest] = node.content ?? []
  const text = [first ? block(first) : '', ...rest.map((child) => block(child))].filter(Boolean).join('\n')
  return indent(text, ' '.repeat(marker.length), marker)
}

function block(node: JSONContent): string {
  const children = node.content ?? []
  switch (node.type) {
    case 'paragraph':
      return escapeLineStart(inline(children))
    case 'heading':
      return `${'#'.repeat(Math.min(Math.max(Number(node.attrs?.level ?? 2), 1), 6))} ${inline(children)}`
    case 'blockquote':
      return blocks(children)
        .split('\n')
        .map((line) => (line === '' ? '>' : `> ${line}`))
        .join('\n')
    case 'codeBlock':
      return `\`\`\`${node.attrs?.language ?? ''}\n${children.map((child) => child.text ?? '').join('')}\n\`\`\``
    case 'horizontalRule':
      return '---'
    case 'bulletList':
      return children.map((item) => listItem(item, '- ')).join('\n')
    case 'orderedList':
      return children.map((item, index) => listItem(item, `${Number(node.attrs?.start ?? 1) + index}. `)).join('\n')
    default:
      return inline(children)
  }
}

function blocks(nodes: JSONContent[]): string {
  return nodes.map((node) => block(node)).join('\n\n')
}

export function docToMarkdown(doc: JSONContent): string {
  return blocks(doc.content ?? [])
    .replace(/\n{3,}/g, '\n\n')
    .trim()
}
