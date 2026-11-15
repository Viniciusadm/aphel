import { useTranslation } from 'react-i18next'
const ACTIONS = {'activity.created':'created','activity.completed':'completed','activity.updated':'updated','activity.deleted':'deleted','activity.restored':'restored','activity.archived':'archived','activity.unarchived':'unarchived'} as const
export function TasksDashboardEventLabel({ event }: { event: { event_type: string; payload: Record<string, unknown> | null } }) {
  const fallback = useTranslation('dashboard').t('events.fallback')
  const { t } = useTranslation('tasks', { keyPrefix: 'dashboard' })
  const action = ACTIONS[event.event_type as keyof typeof ACTIONS]
  return action ? t(`events.${action}`) : fallback
}
