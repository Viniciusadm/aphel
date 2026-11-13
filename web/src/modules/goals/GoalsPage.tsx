import { PaginationSummary } from '@/components/ui/pagination-summary'
import { Plus, Target } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { Pagination } from '@/components/ui/pagination'
import { SegmentedControl } from '@/components/ui/segmented-control'
import { cn } from '@/lib/utils'
import {
  GOAL_OWNER_TYPES,
  GOALS_PER_PAGE,
  goalsApi,
  useGoalOwners,
  useGoals,
  useInvalidateGoals,
  type Goal,
  type GoalOwnerType,
} from './api'
import { GoalCard } from './GoalCard'
import { GoalDialog } from './GoalDialog'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

type GoalView = 'current' | 'all'

export function GoalsPage() {
  const { t } = useTranslation('goals')
  const [view, setView] = useState<GoalView>('current')
  const [ownerType, setOwnerType] = useState<GoalOwnerType | ''>('')
  const [page, setPage] = useState(1)
  const owners = useGoalOwners()
  const goals = useGoals({
    current: view === 'current' ? true : undefined,
    owner_type: ownerType || undefined,
    page,
    per_page: GOALS_PER_PAGE,
  })
  const invalidate = useInvalidateGoals()
  const [editor, setEditor] = useState<{ goal: Goal | null } | null>(null)
  const [deleting, setDeleting] = useState<Goal | null>(null)
  const canCreate = owners.data?.can_manage ?? false

  const remove = async (goal: Goal) => {
    await goalsApi.remove(goal.id)
    await invalidate()
    notifySuccess(t('toast.deleted'))
  }

  const newButton = canCreate ? (
    <Button size="sm" onClick={() => setEditor({ goal: null })}>
      <Plus aria-hidden="true" />
      {t('page.new')}
    </Button>
  ) : undefined

  let body
  if (goals.isPending) {
    body = (
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3" aria-busy="true">
        <Skeleton className="h-40 w-full" />
        <Skeleton className="h-40 w-full" />
        <Skeleton className="h-40 w-full" />
      </div>
    )
  } else if (goals.isError) {
    body = <ErrorState onRetry={() => void goals.refetch()} />
  } else if (goals.data.data.length === 0) {
    body = (
      <EmptyState
        icon={Target}
        title={t(view === 'current' ? 'page.emptyCurrentTitle' : 'page.emptyTitle')}
        description={t(canCreate ? 'page.emptyDescription' : 'page.emptyReadOnly')}
        action={newButton}
      />
    )
  } else {
    body = (
      <>
        <PaginationSummary page={page} perPage={GOALS_PER_PAGE} total={goals.data.meta.total} />
        <ul
          className={cn('grid gap-3 sm:grid-cols-2 xl:grid-cols-3 transition-opacity', goals.isFetching && 'opacity-60')}
          aria-label={t('page.listLabel')}
        >
          {goals.data.data.map((goal) => (
            <li key={goal.id} className="min-w-0">
              <GoalCard goal={goal} onEdit={(target) => setEditor({ goal: target })} onDelete={setDeleting} />
            </li>
          ))}
        </ul>
        <Pagination page={page} perPage={GOALS_PER_PAGE} total={goals.data.meta.total} onPageChange={setPage} />
      </>
    )
  }

  return (
    <Page>
<PageHeader title={t('page.title')} description={t('page.description')} actions={<>
        {newButton}
      
      </>} />
      <div className="flex flex-wrap items-center justify-between gap-3">
        <SegmentedControl
          aria-label={t('page.viewLabel')}
          value={view}
          onValueChange={(value) => {
            setView(value)
            setPage(1)
          }}
          options={(['current', 'all'] as const).map((candidate) => ({ value: candidate, label: t(`page.views.${candidate}`) }))}
        />
        <SegmentedControl
          aria-label={t('page.ownerFilter')}
          value={ownerType === '' ? 'all' : ownerType}
          onValueChange={(value) => {
            setOwnerType(value === 'all' ? '' : (value as GoalOwnerType))
            setPage(1)
          }}
          options={[
            { value: 'all', label: t('page.allOwners') },
            ...GOAL_OWNER_TYPES.map((type) => ({ value: type, label: t(`ownerTypes.${type}`) })),
          ]}
        />
      </div>
      {body}
      {canCreate && (
        <GoalDialog
          open={editor !== null}
          editing={editor?.goal ?? null}
          onOpenChange={(open) => !open && setEditor(null)}
          onSaved={() => void invalidate()}
        />
      )}
      <ConfirmDialog
        open={deleting !== null}
        onOpenChange={(open) => !open && setDeleting(null)}
        title={t('deleteDialog.title')}
        description={t('deleteDialog.description')}
        confirmLabel={t('deleteDialog.confirm')}
        variant="destructive"
        onConfirm={() => (deleting ? remove(deleting) : undefined)}
      />
    </Page>
  )
}
