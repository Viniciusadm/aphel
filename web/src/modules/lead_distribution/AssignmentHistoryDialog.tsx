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
import { Pagination } from '@/components/ui/pagination'
import { ASSIGNMENTS_PER_PAGE, useDistributionAssignments, type DistributionAssignment, type DistributionRule } from './api'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'

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
      <DialogContent className="sm:max-w-3xl">
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
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead scope="col">
                    {t('history.columns.date')}
                  </TableHead>
                  {!rule && (
                    <TableHead scope="col">
                      {t('history.columns.rule')}
                    </TableHead>
                  )}
                  <TableHead scope="col">
                    {t('history.columns.user')}
                  </TableHead>
                  <TableHead scope="col" className="hidden sm:table-cell">
                    {t('history.columns.record')}
                  </TableHead>
                  <TableHead scope="col" className="hidden md:table-cell">
                    {t('history.columns.source')}
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {assignments.data.data.map((assignment) => {
                  const href = recordHref(assignment)
                  return (
                    <TableRow key={assignment.id}>
                      <TableCell className="whitespace-nowrap tabular-nums">
                        {formatter.format(new Date(assignment.created_at))}
                      </TableCell>
                      {!rule && <TableCell>{assignment.rule_name}</TableCell>}
                      <TableCell>{assignment.user_name ?? t('history.removedUser')}</TableCell>
                      <TableCell className="hidden sm:table-cell">
                        {href ? (
                          <Link className="text-primary underline-offset-4 hover:underline" to={href} onClick={onClose}>
                            {t('history.openRecord')}
                          </Link>
                        ) : (
                          <span className="text-muted-foreground">{t('history.noRecord')}</span>
                        )}
                      </TableCell>
                      <TableCell className="hidden text-muted-foreground md:table-cell">
                        {t(`history.sources.${assignment.source}`)}
                      </TableCell>
                    </TableRow>
                  )
                })}
              </TableBody>
            </Table>
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
