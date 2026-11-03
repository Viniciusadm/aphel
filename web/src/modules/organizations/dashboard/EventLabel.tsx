import { useTranslation } from 'react-i18next'
const ACTIONS = {'company.created':'created'} as const
export function OrganizationsDashboardEventLabel({ event }: { event: { event_type: string; payload: Record<string, unknown> | null } }) {
  const { t } = useTranslation('organizations', { keyPrefix: 'dashboard' })
  const action = ACTIONS[event.event_type as keyof typeof ACTIONS]
  return action ? t(`events.${action}`) : event.event_type
}
