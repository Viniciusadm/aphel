import { AlertCircle } from 'lucide-react'
import type { ValidationIssue } from '../lib/errors'
import { useIssueText } from '../lib/issue-text'

export interface IssueListProps {
  id?: string
  issues: ValidationIssue[]
  showPath?: boolean
}

export function IssueList({ id, issues, showPath = false }: IssueListProps) {
  const text = useIssueText()
  if (issues.length === 0) return null
  return (
    <ul id={id} className="grid gap-1 text-xs text-destructive" role="alert">
      {issues.map((issue, index) => (
        <li key={`${issue.path}-${index}`} className="flex items-start gap-1">
          <AlertCircle className="mt-0.5 size-3 shrink-0" aria-hidden="true" />
          <span className="min-w-0 break-words">
            {showPath && issue.path && <code className="mr-1">{issue.path}</code>}
            {text(issue)}
          </span>
        </li>
      ))}
    </ul>
  )
}
