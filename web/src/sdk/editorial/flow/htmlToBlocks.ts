import { Editor } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'

import { newBlock, type Block, type ImageBlock, type VideoBlock } from '../../blocks/blocks'
import { videoOf } from '../../blocks/validation'
import { BlockCard } from './BlockCard'
import { docToBlocks } from './flowDoc'

export interface HtmlImport {
  blocks: Block[]
  missingImages: string[]
}

function basename(source: string): string {
  const clean = source.split('#')[0].split('?')[0]
  return decodeURIComponent(clean.slice(clean.lastIndexOf('/') + 1)).toLowerCase()
}

function card(block: Block): string {
  const holder = document.createElement('div')
  holder.setAttribute('data-block-card', JSON.stringify(block))
  return holder.outerHTML
}

function imageBlock(image: HTMLImageElement, files: Map<string, string>, caption: string, missing: string[]): string {
  const source = image.getAttribute('src') ?? ''
  const file = files.get(basename(source))
  if (!file) {
    if (source) missing.push(source)
    const text = document.createElement('p')
    const link = document.createElement('a')
    link.href = source
    link.textContent = image.getAttribute('alt') || source
    text.append(link)
    return text.outerHTML
  }
  return card({ ...(newBlock('image') as ImageBlock), image: file, alt: image.getAttribute('alt') ?? '', caption })
}

function videoCard(source: string): string | null {
  return videoOf(source) ? card({ ...(newBlock('video') as VideoBlock), url: source }) : null
}

export function htmlToBlocks(html: string, files: Map<string, string>): HtmlImport {
  const parsed = new DOMParser().parseFromString(html, 'text/html')
  const missing: string[] = []
  for (const figure of Array.from(parsed.querySelectorAll('figure'))) {
    const image = figure.querySelector('img')
    const caption = figure.querySelector('figcaption')?.textContent?.trim() ?? ''
    if (image) figure.replaceWith(parsed.createRange().createContextualFragment(imageBlock(image, files, caption, missing)))
  }
  for (const image of Array.from(parsed.querySelectorAll('img'))) {
    image.replaceWith(parsed.createRange().createContextualFragment(imageBlock(image, files, '', missing)))
  }
  for (const frame of Array.from(parsed.querySelectorAll('iframe'))) {
    const replacement = videoCard(frame.getAttribute('src') ?? '')
    if (replacement) frame.replaceWith(parsed.createRange().createContextualFragment(replacement))
    else frame.remove()
  }
  const editor = new Editor({
    extensions: [StarterKit.configure({ underline: false, strike: false, heading: { levels: [2, 3] } }), BlockCard],
    content: parsed.body.innerHTML,
  })
  const blocks = docToBlocks(editor.getJSON())
  editor.destroy()
  return { blocks, missingImages: missing }
}
