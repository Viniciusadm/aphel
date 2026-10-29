import { Link } from 'react-router'
import { recordHref } from './lib/shared'

interface RecordLinkProps {
  recordType: string
  id: string
  title: string
}

export function RecordLink({ recordType, id, title }: RecordLinkProps) {
  const href = recordHref(recordType, id)
  if (!href) return <span className="font-medium break-words">{title}</span>
  return (
    <Link to={href} className="font-medium break-words underline-offset-2 hover:underline">
      {title}
    </Link>
  )
}
