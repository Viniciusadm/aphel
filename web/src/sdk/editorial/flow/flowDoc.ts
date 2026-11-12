import type { JSONContent } from '@tiptap/react'

import { newBlock, type Block, type CodeBlock, type QuoteBlock, type SpacerBlock, type TextBlock } from '../../blocks/blocks'
import { docToMarkdown, markdownToHtml } from '../../blocks/richtext'

const TEXT_RUN_LIMIT = 15000
const SOURCE_MARK = '— '
const TEXT_NODES = new Set(['paragraph', 'heading', 'bulletList', 'orderedList'])

function escapeHtml(value: string): string {
  return value.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
}

function styled(block: Block): boolean {
  return Object.keys(block.style ?? {}).length > 0
}

export function blockToHtml(block: Block): string {
  if (styled(block)) return cardHtml(block)
  switch (block.type) {
    case 'text':
      return markdownToHtml(block.text)
    case 'quote':
      return `<blockquote>${markdownToHtml(block.text)}${block.source ? `<p>${escapeHtml(SOURCE_MARK + block.source)}</p>` : ''}</blockquote>`
    case 'code':
      return `<pre><code${block.language ? ` class="language-${escapeHtml(block.language)}"` : ''}>${escapeHtml(block.code)}</code></pre>`
    case 'spacer':
      return block.divider ? '<hr>' : cardHtml(block)
    default:
      return cardHtml(block)
  }
}

function cardHtml(block: Block): string {
  return `<div data-block-card="${escapeHtml(JSON.stringify(block))}"></div>`
}

export function blocksToHtml(blocks: Block[]): string {
  return blocks.map(blockToHtml).join('')
}

function textOf(node: JSONContent): string {
  return (node.content ?? []).map((child) => child.text ?? '').join('')
}

function textBlocks(nodes: JSONContent[]): TextBlock[] {
  const blocks: TextBlock[] = []
  let chunk: JSONContent[] = []
  let size = 0
  const flush = () => {
    const text = docToMarkdown({ type: 'doc', content: chunk })
    if (text.trim()) blocks.push({ ...(newBlock('text') as TextBlock), text })
    chunk = []
    size = 0
  }
  for (const node of nodes) {
    const length = docToMarkdown({ type: 'doc', content: [node] }).length
    if (size + length > TEXT_RUN_LIMIT && chunk.length > 0) flush()
    chunk.push(node)
    size += length
  }
  if (chunk.length > 0) flush()
  return blocks
}

function quoteBlock(node: JSONContent): QuoteBlock {
  const children = node.content ?? []
  const last = children[children.length - 1]
  const lastText = last && last.type === 'paragraph' ? textOf(last) : ''
  const hasSource = children.length > 1 && lastText.startsWith(SOURCE_MARK)
  const body = hasSource ? children.slice(0, -1) : children
  return {
    ...(newBlock('quote') as QuoteBlock),
    text: docToMarkdown({ type: 'doc', content: body }),
    source: hasSource ? lastText.slice(SOURCE_MARK.length).trim() : '',
  }
}

export function docToBlocks(doc: JSONContent): Block[] {
  const blocks: Block[] = []
  let run: JSONContent[] = []
  const closeRun = () => {
    blocks.push(...textBlocks(run))
    run = []
  }
  for (const node of doc.content ?? []) {
    if (node.type && TEXT_NODES.has(node.type)) {
      run.push(node)
      continue
    }
    closeRun()
    if (node.type === 'blockquote') blocks.push(quoteBlock(node))
    else if (node.type === 'codeBlock') blocks.push({ ...(newBlock('code') as CodeBlock), code: textOf(node), language: String(node.attrs?.language ?? '') })
    else if (node.type === 'horizontalRule') blocks.push({ ...(newBlock('spacer') as SpacerBlock), divider: true })
    else if (node.type === 'blockCard') {
      try {
        const parsed = JSON.parse(String(node.attrs?.block ?? '{}')) as Block
        if (parsed && typeof parsed.type === 'string') blocks.push(parsed)
      } catch {
        continue
      }
    }
  }
  closeRun()
  return blocks
}

export function wordCount(text: string): number {
  return text.trim() === '' ? 0 : text.trim().split(/\s+/).length
}
