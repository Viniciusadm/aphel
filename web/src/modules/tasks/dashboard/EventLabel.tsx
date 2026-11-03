import { useTranslation } from 'react-i18next'
const ACTIONS = {'activity.created':'created','activity.completed':'completed'} as const
export function TasksDashboardEventLabel({ event }: { event: { event_type: string; payload: Record<string, unknown> | null } }) {
  const { t } = useTranslation('tasks', { keyPrefix: 'dashboard' })
  const action = ACTIONS[event.event_type as keyof typeof ACTIONS]
  return action ? t(`events.${action}`) : event.event_type
}
