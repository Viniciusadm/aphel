import { useMutation, useQueryClient } from '@tanstack/react-query'
import { ListChecks } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback/notify'
import { Badge } from '@/components/ui/badge'
import { Checkbox } from '@/components/ui/checkbox'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { Markdown } from '@/modules/notes/components/Markdown'
import { useRecord } from '@/features/records/detail'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { stageGuidanceApi, stageGuidanceKeys, useDealChecklist } from './api'
import type { DealChecklist, DealChecklistItem } from './types'

const DEAL_RECORD_TYPE = 'deal'

export interface DealStageGuidancePanelProps {
  dealId: string
  readOnly?: boolean
}

function stageOf(record: unknown): string | undefined {
  if (typeof record !== 'object' || record === null) return undefined
  const stage = (record as Record<string, unknown>).stage_id
  return typeof stage === 'string' ? stage : undefined
}

export function DealStageGuidancePanel({ dealId, readOnly = false }: DealStageGuidancePanelProps) {
  const { t } = useTranslation('pipelines', { keyPrefix: 'guidance' })
  const record = useRecord(DEAL_RECORD_TYPE, dealId)
  const stageId = stageOf(record.data)
  const checklist = useDealChecklist(dealId, stageId)
  const queryClient = useQueryClient()
  const toggle = useMutation({
    mutationFn: ({ item, done }: { item: DealChecklistItem; done: boolean }) =>
      stageGuidanceApi.toggle(dealId, item.id, done),
    onSuccess: (updated) => {
      queryClient.setQueryData(stageGuidanceKeys.dealStage(dealId, updated.stage_id), updated)
    },
    onError: (error) => notifyError(error, { title: t('panel.error') }),
  })

  if (checklist.isPending) return <Skeleton className="h-40 w-full" />
  if (checklist.isError) return <ErrorState onRetry={() => void checklist.refetch()} />
  const data = checklist.data
  if (!data.guidance.trim() && data.items.length === 0) {
    return <EmptyState icon={ListChecks} title={t('panel.empty')} />
  }
  return (
    <ChecklistView
      data={data}
      readOnly={readOnly}
      pending={toggle.isPending}
      onToggle={(item, done) => toggle.mutate({ item, done })}
    />
  )
}

interface ChecklistViewProps {
  data: DealChecklist
  readOnly: boolean
  pending: boolean
  onToggle: (item: DealChecklistItem, done: boolean) => void
}

function ChecklistView({ data, readOnly, pending, onToggle }: ChecklistViewProps) {
  const { t } = useTranslation('pipelines', { keyPrefix: 'guidance' })
  const formatting = useRecordFormatting()
  const done = data.items.filter((item) => item.done).length

  return (
    <section className="grid min-w-0 gap-4" aria-labelledby="deal-stage-guidance-title">
      <div className="grid gap-1">
        <h2 id="deal-stage-guidance-title" className="text-base font-semibold">
          {t('panel.title', { stage: data.stage_name })}
        </h2>
        {data.items.length > 0 && (
          <p className="text-sm text-muted-foreground">{t('panel.progress', { done, total: data.items.length })}</p>
        )}
        {data.required_pending > 0 && (
          <p className="text-sm text-warning" role="status">
            {t('panel.requiredPending', { count: data.required_pending })}
          </p>
        )}
      </div>
      {data.guidance.trim() && (
        <div className="rounded-md border bg-muted/30 p-3">
          <Markdown source={data.guidance} />
        </div>
      )}
      {data.items.length > 0 && (
        <ul className="grid gap-2">
          {data.items.map((item) => {
            const checkboxId = `deal-checklist-${item.id}`
            const when = item.done_at ? formatDateTime(item.done_at, formatting) : ''
            return (
              <li key={item.id} className="flex min-w-0 items-start gap-3 rounded-md border p-3">
                <Checkbox
                  id={checkboxId}
                  checked={item.done}
                  disabled={readOnly || pending}
                  onCheckedChange={(value) => onToggle(item, value === true)}
                  className="mt-0.5"
                />
                <div className="grid min-w-0 gap-1">
                  <label htmlFor={checkboxId} className="flex min-w-0 flex-wrap items-center gap-2 text-sm">
                    <span className={item.done ? 'break-words text-muted-foreground line-through' : 'break-words'}>
                      {item.label}
                    </span>
                    {item.required && <Badge variant="outline">{t('panel.required')}</Badge>}
                  </label>
                  {item.done && when && (
                    <p className="text-xs text-muted-foreground">
                      {item.done_by?.display_name
                        ? t('panel.doneBy', { name: item.done_by.display_name, date: when })
                        : t('panel.doneAt', { date: when })}
                    </p>
                  )}
                </div>
              </li>
            )
          })}
        </ul>
      )}
    </section>
  )
}
