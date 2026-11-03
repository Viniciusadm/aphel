import { parseMarkdown } from './parse'

export interface MarkdownProps {
  source: string
  className?: string
}

export function Markdown({ source, className }: MarkdownProps) {
  return (
    <div className={['grid gap-2 text-sm break-words', className].filter(Boolean).join(' ')}>
      {parseMarkdown(source)}
    </div>
  )
}
