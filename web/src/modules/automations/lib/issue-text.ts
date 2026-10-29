import { useTranslation } from 'react-i18next'
import type { ValidationIssue } from './errors'

const KNOWN_MESSAGES: Record<string, string> = {
  'is required': 'issues.required',
  'is not a known action': 'issues.unknownAction',
  'is not a known trigger': 'issues.unknownTrigger',
  'must have at least one action to be published': 'issues.needsAction',
  'is not reachable from the start step': 'issues.unreachable',
}

export function useIssueText() {
  const { t } = useTranslation('automations')
  return (issue: ValidationIssue) => {
    const key = KNOWN_MESSAGES[issue.message]
    return key ? t(key as 'issues.required') : issue.message
  }
}
