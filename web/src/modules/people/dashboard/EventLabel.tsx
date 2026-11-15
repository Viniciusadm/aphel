import { useTranslation } from 'react-i18next'
const ACTIONS = {'contact.created':'created','contact.updated':'updated','contact.deleted':'deleted','contact.restored':'restored','contact.archived':'archived','contact.unarchived':'unarchived'} as const
export function PeopleDashboardEventLabel({ event }: { event: { event_type: string; payload: Record<string, unknown> | null } }) {
  const fallback = useTranslation('dashboard').t('events.fallback')
  const { t } = useTranslation('people', { keyPrefix: 'dashboard' })
  const action = ACTIONS[event.event_type as keyof typeof ACTIONS]
  return action ? t(`events.${action}`) : fallback
}
