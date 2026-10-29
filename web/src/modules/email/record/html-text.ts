const BLOCK_TAGS = new Set(['P', 'DIV', 'LI', 'BLOCKQUOTE', 'H1', 'H2', 'H3', 'H4', 'H5', 'H6', 'TR', 'UL', 'OL'])

function collect(node: Node, output: string[]): void {
  if (node.nodeType === Node.TEXT_NODE) {
    output.push(node.textContent ?? '')
    return
  }
  if (!(node instanceof Element)) return
  if (node.tagName === 'BR') {
    output.push('\n')
    return
  }
  if (node.tagName === 'SCRIPT' || node.tagName === 'STYLE') return
  const block = BLOCK_TAGS.has(node.tagName)
  if (block && output.length > 0 && !output[output.length - 1].endsWith('\n')) output.push('\n')
  if (node.tagName === 'LI') output.push('• ')
  node.childNodes.forEach((child) => collect(child, output))
  if (node.tagName === 'A') {
    const href = node.getAttribute('href')
    const text = node.textContent ?? ''
    if (href && !href.startsWith('mailto:') && href !== text) output.push(` (${href})`)
  }
  if (block) output.push('\n')
}

export function htmlToText(html: string): string {
  if (!html.trim()) return ''
  const document = new DOMParser().parseFromString(html, 'text/html')
  const output: string[] = []
  collect(document.body, output)
  return output
    .join('')
    .replace(/\n{3,}/g, '\n\n')
    .trim()
}
