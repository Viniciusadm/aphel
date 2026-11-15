import { useTranslation } from 'react-i18next'
const ACTIONS = {'deal.created':'created','deal.stage_changed':'stage_changed','deal.won':'won','deal.lost':'lost','deal.reopened':'reopened','deal.owner_changed':'owner_changed','deal.updated':'updated','deal.deleted':'deleted','deal.restored':'restored','deal.archived':'archived','deal.unarchived':'unarchived'} as const
export function DealsDashboardEventLabel({ event }: { event: { event_type: string; payload: Record<string, unknown> | null } }) {
  const fallback = useTranslation('dashboard').t('events.fallback')
  const { t } = useTranslation('deals', { keyPrefix: 'dashboard' })
  const action = ACTIONS[event.event_type as keyof typeof ACTIONS]
  const from = event.payload?.from as { name?: string } | undefined
  const to = event.payload?.to as { name?: string } | undefined
  return <>{action ? t(`events.${action}`) : fallback}{event.event_type === 'deal.stage_changed' && from?.name && to?.name ? ` (${t('stageChange', { from: from.name, to: to.name })})` : null}</>
}
