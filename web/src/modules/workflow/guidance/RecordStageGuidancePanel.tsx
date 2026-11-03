import { useMutation, useQueryClient } from '@tanstack/react-query'
import { ListChecks } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback/notify'
import { Badge } from '@/components/ui/badge'
import { Checkbox } from '@/components/ui/checkbox'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { Markdown } from '@/components/markdown'
import { useRecord } from '@/features/records/detail'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { stageGuidanceApi, stageGuidanceKeys, useRecordChecklist } from './api'
import type { RecordChecklist, RecordChecklistItem } from './types'

import { stageField } from '../traits'

export interface RecordStageGuidancePanelProps {
  recordType: string
  recordId: string
  readOnly?: boolean
}

function stageOf(record: unknown, field: string): string | undefined {
  if (typeof record !== 'object' || record === null) return undefined
  const values = record as Record<string, unknown>
  const custom = values.custom_fields
  const stage = values[field] ?? (typeof custom === 'object' && custom !== null ? (custom as Record<string, unknown>)[field] : undefined)
  return typeof stage === 'string' ? stage : undefined
}

export function RecordStageGuidancePanel({ recordType, recordId, readOnly = false }: RecordStageGuidancePanelProps) {
  const { t } = useTranslation('workflow', { keyPrefix: 'guidance' })
  const record = useRecord(recordType, recordId)
  const stageId = stageOf(record.data, stageField(recordType))
  const checklist = useRecordChecklist(recordType, recordId, stageId)
  const queryClient = useQueryClient()
  const toggle = useMutation({
    mutationFn: ({ item, done }: { item: RecordChecklistItem; done: boolean }) =>
      stageGuidanceApi.toggle(recordType, recordId, item.id, done),
    onSuccess: (updated) => {
      queryClient.setQueryData(stageGuidanceKeys.recordStage(recordType, recordId, updated.stage_id), updated)
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
  data: RecordChecklist
  readOnly: boolean
  pending: boolean
  onToggle: (item: RecordChecklistItem, done: boolean) => void
}

function ChecklistView({ data, readOnly, pending, onToggle }: ChecklistViewProps) {
  const { t } = useTranslation('workflow', { keyPrefix: 'guidance' })
  const formatting = useRecordFormatting()
  const done = data.items.filter((item) => item.done).length

  return (
    <section className="grid min-w-0 gap-4" aria-labelledby="record-stage-guidance-title">
      <div className="grid gap-1">
        <h2 id="record-stage-guidance-title" className="text-base font-semibold">
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
            const checkboxId = `record-checklist-${item.id}`
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
