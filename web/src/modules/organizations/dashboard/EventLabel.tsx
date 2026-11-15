import { useTranslation } from 'react-i18next'
const ACTIONS = {'company.created':'created','company.updated':'updated','company.deleted':'deleted','company.restored':'restored','company.archived':'archived','company.unarchived':'unarchived'} as const
export function OrganizationsDashboardEventLabel({ event }: { event: { event_type: string; payload: Record<string, unknown> | null } }) {
  const fallback = useTranslation('dashboard').t('events.fallback')
  const { t } = useTranslation('organizations', { keyPrefix: 'dashboard' })
  const action = ACTIONS[event.event_type as keyof typeof ACTIONS]
  return action ? t(`events.${action}`) : fallback
}
