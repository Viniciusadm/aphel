import { ShieldCheck } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { cn } from '@/lib/utils'
import { useApprovalInbox } from './api'
import { ApprovalStatusBadge } from './components/StatusBadge'
import {
  APPROVAL_BOXES,
  APPROVAL_STATUSES,
  APPROVALS_MANAGE_PERMISSION,
  APPROVALS_PER_PAGE,
  approvalPath,
} from './constants'
import { useRecordTypeLabel } from './lib/labels'
import { recordPath } from './lib/record-link'
import type { ApprovalBox, ApprovalRequest, ApprovalStatus } from './types'
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Page } from '@/components/ui/page'
import { Field } from '@/components/ui/field'

export function ApprovalsPage() {
  const { t } = useTranslation('approvals')
  const { hasPermission } = useSession()
  const statusId = useId()
  const canManage = hasPermission(APPROVALS_MANAGE_PERMISSION)
  const boxes = APPROVAL_BOXES.filter((box) => box !== 'all' || canManage)
  const [box, setBox] = useState<ApprovalBox>('pending_for_me')
  const [status, setStatus] = useState<ApprovalStatus | ''>('pending')
  const [page, setPage] = useState(1)
  const inbox = useApprovalInbox({ box, status: status || undefined, page })

  let body
  if (inbox.isPending) {
    body = (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-14 w-full" />
        <Skeleton className="h-14 w-full" />
        <Skeleton className="h-14 w-full" />
      </div>
    )
  } else if (inbox.isError) {
    body = <ErrorState onRetry={() => void inbox.refetch()} />
  } else if (inbox.data.data.length === 0) {
    body = <EmptyState icon={ShieldCheck} title={t('inbox.emptyTitle')} description={t(`inbox.empty.${box}`)} />
  } else {
    const pages = Math.max(1, Math.ceil(inbox.data.meta.total / APPROVALS_PER_PAGE))
    body = (
      <>
        <RequestsTable requests={inbox.data.data} />
        {pages > 1 && (
          <nav
            aria-label={t('pager.label')}
            className="flex flex-wrap items-center justify-between gap-2 text-sm text-muted-foreground"
          >
            <span>
              {t('pager.summary', {
                page,
                pages,
                total: inbox.data.meta.total,
              })}
            </span>
            <div className="flex gap-2">
              <Button variant="outline" size="sm" onClick={() => setPage(page - 1)} disabled={page <= 1}>
                {t('pager.previous')}
              </Button>
              <Button variant="outline" size="sm" onClick={() => setPage(page + 1)} disabled={page >= pages}>
                {t('pager.next')}
              </Button>
            </div>
          </nav>
        )}
      </>
    )
  }

  return (
    <Page>
      <PageHeader title={t('inbox.title')} description={t('inbox.description')} />
      <div className="flex flex-wrap items-center justify-between gap-3">
        <NativeSelect aria-label={t('inbox.title')} value={box} onChange={(event) => { setBox(event.target.value as ApprovalBox); setPage(1) }}>
          {boxes.map((item) => <option key={item} value={item}>{t(`inbox.boxes.${item}`)}</option>)}
        </NativeSelect>
        <Field id={statusId} label={t('inbox.status')} inline>
          <NativeSelect
            id={statusId}
            className="w-44"
            value={status}
            onChange={(event) => {
              setStatus(event.target.value as ApprovalStatus | '')
              setPage(1)
            }}
          >
            <option value="">{t('inbox.allStatuses')}</option>
            {APPROVAL_STATUSES.map((item) => (
              <option key={item} value={item}>
                {t(`status.${item}`)}
              </option>
            ))}
          </NativeSelect>
        </Field>
      </div>
      {body}
    </Page>
  )
}

function RequestsTable({ requests }: { requests: ApprovalRequest[] }) {
  const { t } = useTranslation('approvals')
  const formatting = useRecordFormatting()
  const recordTypeLabel = useRecordTypeLabel()
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead scope="col">
            {t('columns.record')}
          </TableHead>
          <TableHead scope="col">
            {t('columns.policy')}
          </TableHead>
          <TableHead scope="col" className="hidden md:table-cell">
            {t('columns.requestedBy')}
          </TableHead>
          <TableHead scope="col" className="hidden sm:table-cell">
            {t('columns.dueAt')}
          </TableHead>
          <TableHead scope="col">
            {t('columns.status')}
          </TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {requests.map((request) => {
          const link = recordPath(request.record_type, request.record_id)
          const title = request.record_title?.trim() || t('request.untitledRecord')
          const overdue = request.is_overdue && request.status === 'pending'
          return (
            <TableRow key={request.id} className="align-top">
              <TableCell>
                {link ? (
                  <Link to={link} className="font-medium hover:underline">
                    {title}
                  </Link>
                ) : (
                  <span className="font-medium">{title}</span>
                )}
                <div className="text-xs text-muted-foreground">{recordTypeLabel(request.record_type)}</div>
              </TableCell>
              <TableCell>
                <Link to={approvalPath(request.id)} className="hover:underline">
                  {request.policy_name}
                </Link>
                <div className="text-xs text-muted-foreground">
                  {t(`moments.${request.moment}`)}
                  {request.target_name && ` · ${request.target_name}`}
                </div>
              </TableCell>
              <TableCell className="hidden md:table-cell">
                {request.requested_by?.display_name ?? t('request.system')}
                <div className="text-xs text-muted-foreground">
                  {formatDateTime(request.requested_at, formatting)}
                </div>
              </TableCell>
              <TableCell className={cn('hidden px-4 py-3 sm:table-cell', overdue && 'font-medium text-destructive')}>
                {request.due_at ? formatDateTime(request.due_at, formatting) : '—'}
                {overdue && <div className="text-xs">{t('request.overdue')}</div>}
              </TableCell>
              <TableCell>
                <ApprovalStatusBadge status={request.status} />
              </TableCell>
            </TableRow>
          )
        })}
      </TableBody>
    </Table>
  )
}
