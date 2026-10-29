import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { getRecordResource } from '@/features/records/resources'
import { ListSkeleton } from '@/features/settings/components/ListSkeleton'
import { Pagination } from '@/features/settings/components/Pagination'
import { ASSIGNMENTS_PER_PAGE, useDistributionAssignments, type DistributionAssignment, type DistributionRule } from './api'

interface AssignmentHistoryDialogProps {
  rule: DistributionRule | null
  onClose: () => void
}

function recordHref(assignment: DistributionAssignment): string | null {
  if (!assignment.record_type || !assignment.record_id) return null
  return getRecordResource(assignment.record_type)?.detailPath?.(assignment.record_id) ?? null
}

export function AssignmentHistoryDialog({ rule, onClose }: AssignmentHistoryDialogProps) {
  const { t, i18n } = useTranslation('lead_distribution')
  const [page, setPage] = useState(1)
  const assignments = useDistributionAssignments(rule?.id ?? null, page)
  const formatter = useMemo(
    () => new Intl.DateTimeFormat(i18n.language, { dateStyle: 'short', timeStyle: 'short' }),
    [i18n.language],
  )

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>{rule ? t('history.ruleTitle', { name: rule.name }) : t('history.title')}</DialogTitle>
          <DialogDescription>{t('history.description')}</DialogDescription>
        </DialogHeader>
        {assignments.isPending ? (
          <ListSkeleton />
        ) : assignments.isError ? (
          <ErrorState onRetry={() => void assignments.refetch()} />
        ) : assignments.data.data.length === 0 ? (
          <EmptyState title={t('history.empty')} />
        ) : (
          <div className="grid gap-3">
            <div className="relative overflow-x-auto rounded-lg border">
              <table className="w-full text-sm">
                <thead className="bg-muted/50 text-left text-muted-foreground">
                  <tr>
                    <th scope="col" className="px-3 py-2 font-medium">
                      {t('history.columns.date')}
                    </th>
                    {!rule && (
                      <th scope="col" className="px-3 py-2 font-medium">
                        {t('history.columns.rule')}
                      </th>
                    )}
                    <th scope="col" className="px-3 py-2 font-medium">
                      {t('history.columns.user')}
                    </th>
                    <th scope="col" className="hidden px-3 py-2 font-medium sm:table-cell">
                      {t('history.columns.record')}
                    </th>
                    <th scope="col" className="hidden px-3 py-2 font-medium md:table-cell">
                      {t('history.columns.source')}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {assignments.data.data.map((assignment) => {
                    const href = recordHref(assignment)
                    return (
                      <tr key={assignment.id} className="border-t">
                        <td className="whitespace-nowrap px-3 py-2 tabular-nums">
                          {formatter.format(new Date(assignment.created_at))}
                        </td>
                        {!rule && <td className="px-3 py-2">{assignment.rule_name}</td>}
                        <td className="px-3 py-2">{assignment.user_name ?? t('history.removedUser')}</td>
                        <td className="hidden px-3 py-2 sm:table-cell">
                          {href ? (
                            <Link className="text-primary underline-offset-4 hover:underline" to={href} onClick={onClose}>
                              {t('history.openRecord')}
                            </Link>
                          ) : (
                            <span className="text-muted-foreground">{t('history.noRecord')}</span>
                          )}
                        </td>
                        <td className="hidden px-3 py-2 text-muted-foreground md:table-cell">
                          {t(`history.sources.${assignment.source}`)}
                        </td>
                      </tr>
                    )
                  })}
                </tbody>
              </table>
            </div>
            <Pagination
              page={page}
              perPage={ASSIGNMENTS_PER_PAGE}
              total={assignments.data.meta.total}
              onPageChange={setPage}
            />
          </div>
        )}
      </DialogContent>
    </Dialog>
  )
}
