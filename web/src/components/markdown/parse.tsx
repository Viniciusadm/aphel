import { Fragment, type ReactNode } from 'react'

const SAFE_PROTOCOLS = ['http:', 'https:', 'mailto:']

const INLINE_PATTERN =
  /(`[^`\n]+`)|(\*\*[^*\n]+\*\*|(?<!\w)__[^_\n]+__(?!\w))|(~~[^~\n]+~~)|(\*[^*\n]+\*|(?<!\w)_[^_\n]+_(?!\w))|(\[[^\]\n]+\]\([^)\s]+\))|(https?:\/\/[^\s<>()]+[^\s<>().,;:!?'"])/

export function safeHref(raw: string): string | null {
  const value = raw.trim()
  try {
    const url = new URL(value)
    return SAFE_PROTOCOLS.includes(url.protocol) ? url.href : null
  } catch {
    return null
  }
}

function link(href: string, children: ReactNode, key: string): ReactNode {
  const safe = safeHref(href)
  if (!safe) return <Fragment key={key}>{children}</Fragment>
  return (
    <a
      key={key}
      href={safe}
      target="_blank"
      rel="noopener noreferrer nofollow"
      className="text-primary underline underline-offset-4 break-all"
    >
      {children}
    </a>
  )
}

export function parseInline(text: string, keyPrefix = 'i'): ReactNode[] {
  const nodes: ReactNode[] = []
  let rest = text
  let index = 0
  while (rest.length > 0) {
    const match = INLINE_PATTERN.exec(rest)
    if (!match) {
      nodes.push(rest)
      break
    }
    if (match.index > 0) nodes.push(rest.slice(0, match.index))
    const token = match[0]
    const key = `${keyPrefix}-${index}`
    if (match[1]) {
      nodes.push(
        <code key={key} className="rounded bg-muted px-1 py-0.5 font-mono text-[0.85em]">
          {token.slice(1, -1)}
        </code>,
      )
    } else if (match[2]) {
      nodes.push(<strong key={key}>{parseInline(token.slice(2, -2), key)}</strong>)
    } else if (match[3]) {
      nodes.push(<del key={key}>{parseInline(token.slice(2, -2), key)}</del>)
    } else if (match[4]) {
      nodes.push(<em key={key}>{parseInline(token.slice(1, -1), key)}</em>)
    } else if (match[5]) {
      const separator = token.indexOf('](')
      nodes.push(link(token.slice(separator + 2, -1), parseInline(token.slice(1, separator), key), key))
    } else {
      nodes.push(link(token, token, key))
    }
    rest = rest.slice(match.index + token.length)
    index += 1
  }
  return nodes
}

function inlineLines(lines: string[], key: string): ReactNode[] {
  return lines.flatMap((line, index) => {
    const content = parseInline(line, `${key}-${index}`)
    return index === 0 ? content : [<br key={`${key}-br-${index}`} />, ...content]
  })
}

const HEADING = /^(#{1,6})\s+(.*)$/
const UNORDERED = /^\s*[-*+]\s+(.*)$/
const ORDERED = /^\s*\d+[.)]\s+(.*)$/
const BLOCKQUOTE = /^\s*>\s?(.*)$/
const RULE = /^\s*(?:-{3,}|\*{3,}|_{3,})\s*$/
const FENCE = /^\s*```/

const HEADING_CLASSES = ['text-base font-semibold', 'text-sm font-semibold', 'text-sm font-medium']

export function parseMarkdown(source: string): ReactNode[] {
  const lines = source.replace(/\r\n?/g, '\n').split('\n')
  const blocks: ReactNode[] = []
  let index = 0
  const key = () => `b-${blocks.length}`

  while (index < lines.length) {
    const line = lines[index] ?? ''
    if (line.trim() === '') {
      index += 1
      continue
    }
    if (FENCE.test(line)) {
      const code: string[] = []
      index += 1
      while (index < lines.length && !FENCE.test(lines[index] ?? '')) {
        code.push(lines[index] ?? '')
        index += 1
      }
      index += 1
      blocks.push(
        <pre key={key()} className="overflow-x-auto rounded-md bg-muted p-3 font-mono text-xs">
          <code>{code.join('\n')}</code>
        </pre>,
      )
      continue
    }
    const heading = HEADING.exec(line)
    if (heading) {
      const level = Math.min((heading[1] ?? '#').length, 3) - 1
      const Tag = (['h3', 'h4', 'h5'] as const)[level] ?? 'h5'
      blocks.push(
        <Tag key={key()} className={HEADING_CLASSES[level]}>
          {parseInline(heading[2] ?? '', key())}
        </Tag>,
      )
      index += 1
      continue
    }
    if (RULE.test(line)) {
      blocks.push(<hr key={key()} className="border-border" />)
      index += 1
      continue
    }
    if (UNORDERED.test(line) || ORDERED.test(line)) {
      const ordered = ORDERED.test(line) && !UNORDERED.test(line)
      const pattern = ordered ? ORDERED : UNORDERED
      const items: string[] = []
      while (index < lines.length && pattern.test(lines[index] ?? '')) {
        items.push(pattern.exec(lines[index] ?? '')?.[1] ?? '')
        index += 1
      }
      const blockKey = key()
      const children = items.map((item, itemIndex) => (
        <li key={`${blockKey}-${itemIndex}`}>{parseInline(item, `${blockKey}-${itemIndex}`)}</li>
      ))
      blocks.push(
        ordered ? (
          <ol key={blockKey} className="list-decimal pl-5">
            {children}
          </ol>
        ) : (
          <ul key={blockKey} className="list-disc pl-5">
            {children}
          </ul>
        ),
      )
      continue
    }
    if (BLOCKQUOTE.test(line)) {
      const quoted: string[] = []
      while (index < lines.length && BLOCKQUOTE.test(lines[index] ?? '')) {
        quoted.push(BLOCKQUOTE.exec(lines[index] ?? '')?.[1] ?? '')
        index += 1
      }
      const blockKey = key()
      blocks.push(
        <blockquote key={blockKey} className="border-l-2 pl-3 text-muted-foreground">
          {inlineLines(quoted, blockKey)}
        </blockquote>,
      )
      continue
    }
    const paragraph: string[] = []
    while (index < lines.length) {
      const current = lines[index] ?? ''
      if (
        current.trim() === '' ||
        FENCE.test(current) ||
        HEADING.test(current) ||
        RULE.test(current) ||
        UNORDERED.test(current) ||
        ORDERED.test(current) ||
        BLOCKQUOTE.test(current)
      ) {
        break
      }
      paragraph.push(current)
      index += 1
    }
    const blockKey = key()
    blocks.push(<p key={blockKey}>{inlineLines(paragraph, blockKey)}</p>)
  }
  return blocks
}
