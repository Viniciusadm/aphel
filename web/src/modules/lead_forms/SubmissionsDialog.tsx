import { Inbox } from 'lucide-react'
import { useState } from 'react'
import { Link } from 'react-router'
import { Badge } from '@/components/ui/badge'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { Pagination } from '@/features/settings/components/Pagination'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import {
  LEAD_FORMS_KEY,
  SUBMISSIONS_PER_PAGE,
  SUBMISSION_STATUSES,
  useLeadFormSubmissions,
  type LeadForm,
  type LeadFormSubmission,
  type LeadFormSubmissionStatus,
} from './api'

interface SubmissionsDialogProps {
  form: LeadForm | null
  onClose: () => void
}

export function SubmissionsDialog({ form, onClose }: SubmissionsDialogProps) {
  const t = useModuleTranslation(LEAD_FORMS_KEY)
  return (
    <Dialog open={form !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-h-[92vh] overflow-y-auto sm:max-w-3xl">
        {form && (
          <>
            <DialogHeader>
              <DialogTitle>{t('submissions.title', { name: form.name })}</DialogTitle>
              <DialogDescription>{t('submissions.description')}</DialogDescription>
            </DialogHeader>
            <SubmissionList form={form} />
          </>
        )}
      </DialogContent>
    </Dialog>
  )
}

const STATUS_VARIANT = {
  accepted: 'success',
  spam: 'outline',
  failed: 'destructive',
} as const

function SubmissionList({ form }: { form: LeadForm }) {
  const t = useModuleTranslation(LEAD_FORMS_KEY)
  const [status, setStatus] = useState<LeadFormSubmissionStatus | null>(null)
  const [page, setPage] = useState(1)
  const submissions = useLeadFormSubmissions(form.id, status, page)

  return (
    <div className="grid gap-3">
      <NativeSelect
        aria-label={t('submissions.statusFilter')}
        className="sm:max-w-xs"
        value={status ?? ''}
        onChange={(event) => {
          setStatus((event.target.value || null) as LeadFormSubmissionStatus | null)
          setPage(1)
        }}
      >
        <option value="">{t('submissions.allStatuses')}</option>
        {SUBMISSION_STATUSES.map((item) => (
          <option key={item} value={item}>
            {t(`status.${item}`)}
          </option>
        ))}
      </NativeSelect>
      {submissions.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-16 w-full" />
          <Skeleton className="h-16 w-full" />
        </div>
      ) : submissions.isError ? (
        <ErrorState onRetry={() => void submissions.refetch()} />
      ) : submissions.data.data.length === 0 ? (
        <EmptyState icon={Inbox} title={t('submissions.emptyTitle')} description={t('submissions.emptyDescription')} />
      ) : (
        <>
          <ul className="grid gap-2" aria-label={t('submissions.listLabel')}>
            {submissions.data.data.map((submission) => (
              <SubmissionRow key={submission.id} form={form} submission={submission} />
            ))}
          </ul>
          <Pagination
            page={page}
            perPage={SUBMISSIONS_PER_PAGE}
            total={submissions.data.meta.total}
            onPageChange={setPage}
          />
        </>
      )}
    </div>
  )
}

function displayValue(value: unknown, yes: string, no: string): string {
  if (typeof value === 'boolean') return value ? yes : no
  if (value === null || value === undefined) return ''
  return typeof value === 'string' ? value : JSON.stringify(value)
}

function SubmissionRow({ form, submission }: { form: LeadForm; submission: LeadFormSubmission }) {
  const t = useModuleTranslation(LEAD_FORMS_KEY)
  const formatting = useRecordFormatting()
  const data = (submission.data ?? {}) as Record<string, unknown>
  const labels = new Map((form.fields ?? []).map((field) => [field.key, field.label]))
  const entries = Object.entries(data).filter(([, value]) => displayValue(value, '', '') !== '')

  return (
    <li className="grid gap-2 rounded-md border bg-card px-3 py-2 text-sm">
      <div className="flex flex-wrap items-center gap-2">
        <Badge variant={STATUS_VARIANT[submission.status]}>{t(`status.${submission.status}`)}</Badge>
        <span>{formatDateTime(submission.created_at, formatting)}</span>
        {submission.ip_address && <code className="text-xs text-muted-foreground">{submission.ip_address}</code>}
        {submission.origin && <span className="text-xs text-muted-foreground break-all">{submission.origin}</span>}
      </div>
      {entries.length > 0 && (
        <dl className="grid gap-x-3 gap-y-0.5 text-xs sm:grid-cols-[max-content_minmax(0,1fr)]">
          {entries.map(([key, value]) => (
            <div key={key} className="contents">
              <dt className="font-medium text-muted-foreground">
                {key === '_aphel_hp' ? t('submissions.honeypot') : (labels.get(key) ?? key)}
              </dt>
              <dd className="break-words whitespace-pre-wrap">
                {displayValue(value, t('submissions.yes'), t('submissions.no'))}
              </dd>
            </div>
          ))}
        </dl>
      )}
      <div className="flex flex-wrap gap-3 text-xs">
        {submission.contact_id && (
          <Link className="text-primary hover:underline" to={`/contacts/${encodeURIComponent(submission.contact_id)}`}>
            {submission.contact_created ? t('submissions.contactCreated') : t('submissions.contactReused')}
          </Link>
        )}
        {submission.deal_id && (
          <Link className="text-primary hover:underline" to={`/deals/${encodeURIComponent(submission.deal_id)}`}>
            {t('submissions.deal')}
          </Link>
        )}
      </div>
      {submission.error_code && (
        <p className="text-xs text-destructive break-words">
          {t(`errors.submission.${submission.error_code}`, { defaultValue: submission.error_code })}
          {submission.error_message ? ` — ${submission.error_message}` : ''}
        </p>
      )}
    </li>
  )
}
