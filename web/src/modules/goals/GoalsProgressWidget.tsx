import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { EmptyState } from '@/components/ui/empty-state'
import type { Goal } from './api'
import { GoalCard } from './GoalCard'

export interface GoalsProgressData {
  active_on: string
  limit: number
  only_mine: boolean
  user_id: string
  total: number
  items: Goal[]
}

export function GoalsProgressWidget({ data }: { data: GoalsProgressData }) {
  const { t } = useTranslation('goals')
  const items = data.items ?? []
  if (items.length === 0) {
    return <EmptyState className="p-4" title={t('widget.emptyTitle')} description={t('widget.emptyDescription')} />
  }
  return (
    <div className="grid gap-2">
      <ul className="grid gap-2 sm:grid-cols-2" aria-label={t('widget.listLabel')}>
        {items.map((goal) => (
          <li key={goal.id} className="min-w-0">
            <GoalCard goal={goal} compact />
          </li>
        ))}
      </ul>
      {data.total > items.length && (
        <Link to="/goals" className="justify-self-end text-sm font-medium underline-offset-4 hover:underline">
          {t('widget.seeAll', { count: data.total })}
        </Link>
      )}
    </div>
  )
}
