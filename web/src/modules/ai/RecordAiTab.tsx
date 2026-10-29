import { useMutation } from '@tanstack/react-query'
import { CalendarPlus, Check, Lightbulb, Loader2, RefreshCw, Sparkles } from 'lucide-react'
import { useState } from 'react'
import { useSession } from '@/app/session/session-context'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { activitiesApi, useActivityTypes } from '@/modules/activities/api'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import type { RecordTabRenderProps } from '@/kernel/modules/define'
import { useInvalidateActivitiesForRecord } from './activities'
import { AI_KEY, aiApi, useAiStatus, type AiActionSuggestion } from './api'
import { AiErrorMessage, AiFooter } from './AiNotice'
import { Sources } from './Sources'

const TASK_TYPE_KEY = 'task'
const DEFAULT_TASK_HOUR = 9

function dueDate(days: number): string {
  const date = new Date()
  date.setDate(date.getDate() + days)
  date.setHours(DEFAULT_TASK_HOUR, 0, 0, 0)
  return date.toISOString()
}

export function RecordAiTab({ recordType, recordId }: RecordTabRenderProps) {
  const t = useModuleTranslation(AI_KEY)
  const status = useAiStatus()

  if (status.isPending) {
    return (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-24 w-full" />
        <Skeleton className="h-24 w-full" />
      </div>
    )
  }
  if (status.isError) return <ErrorState onRetry={() => void status.refetch()} />
  if (!status.data.configured) {
    return <EmptyState icon={Sparkles} title={t('tab.notConfiguredTitle')} description={t('tab.notConfiguredDescription')} />
  }

  return (
    <div className="flex flex-col gap-4">
      {status.data.features.summary && <SummarySection recordType={recordType} recordId={recordId} />}
      {status.data.features.next_actions && <NextActionsSection recordType={recordType} recordId={recordId} />}
      {!status.data.features.summary && !status.data.features.next_actions && (
        <EmptyState icon={Sparkles} title={t('tab.allOffTitle')} description={t('tab.allOffDescription')} />
      )}
    </div>
  )
}

interface SectionProps {
  recordType: string
  recordId: string
}

function SummarySection({ recordType, recordId }: SectionProps) {
  const t = useModuleTranslation(AI_KEY)
  const summary = useMutation({ mutationFn: () => aiApi.summary(recordType, recordId) })

  return (
    <section className="grid gap-3 rounded-md border bg-card p-4" aria-label={t('summary.title')}>
      <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
        <div className="grid gap-0.5">
          <h2 className="text-base font-semibold">{t('summary.title')}</h2>
          <p className="text-sm text-muted-foreground">{t(`summary.description.${recordType}`)}</p>
        </div>
        <Button size="sm" variant={summary.data ? 'outline' : 'default'} disabled={summary.isPending} onClick={() => summary.mutate()}>
          {summary.isPending ? (
            <Loader2 className="animate-spin" aria-hidden="true" />
          ) : summary.data ? (
            <RefreshCw aria-hidden="true" />
          ) : (
            <Sparkles aria-hidden="true" />
          )}
          {t(summary.data ? 'summary.regenerate' : 'summary.generate')}
        </Button>
      </div>
      {summary.isPending && (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-4 w-full" />
          <Skeleton className="h-4 w-11/12" />
          <Skeleton className="h-4 w-4/5" />
        </div>
      )}
      {summary.isError && <AiErrorMessage error={summary.error} />}
      {summary.data && !summary.isPending && (
        <div className="grid gap-2">
          <p className="text-sm leading-relaxed whitespace-pre-line">{summary.data.text}</p>
          <Sources sources={summary.data.sources} />
          <AiFooter provider={summary.data.provider} model={summary.data.model} />
        </div>
      )}
    </section>
  )
}

function NextActionsSection({ recordType, recordId }: SectionProps) {
  const t = useModuleTranslation(AI_KEY)
  const actions = useMutation({ mutationFn: () => aiApi.nextActions(recordType, recordId) })
  const [created, setCreated] = useState<Set<number>>(new Set())

  return (
    <section className="grid gap-3 rounded-md border bg-card p-4" aria-label={t('actions.title')}>
      <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
        <div className="grid gap-0.5">
          <h2 className="text-base font-semibold">{t('actions.title')}</h2>
          <p className="text-sm text-muted-foreground">{t('actions.description')}</p>
        </div>
        <Button
          size="sm"
          variant="outline"
          disabled={actions.isPending}
          onClick={() => {
            setCreated(new Set())
            actions.mutate()
          }}
        >
          {actions.isPending ? <Loader2 className="animate-spin" aria-hidden="true" /> : <Lightbulb aria-hidden="true" />}
          {t(actions.data ? 'actions.regenerate' : 'actions.generate')}
        </Button>
      </div>
      {actions.isPending && (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-14 w-full" />
          <Skeleton className="h-14 w-full" />
        </div>
      )}
      {actions.isError && <AiErrorMessage error={actions.error} />}
      {actions.data && !actions.isPending && (
        <div className="grid gap-2">
          {actions.data.suggestions.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t('actions.empty')}</p>
          ) : (
            <ul className="grid gap-2">
              {actions.data.suggestions.map((suggestion, index) => (
                <SuggestionRow
                  key={`${index}-${suggestion.title}`}
                  suggestion={suggestion}
                  recordType={recordType}
                  recordId={recordId}
                  created={created.has(index)}
                  onCreated={() => setCreated((current) => new Set(current).add(index))}
                />
              ))}
            </ul>
          )}
          <Sources sources={actions.data.sources} />
          <AiFooter provider={actions.data.provider} model={actions.data.model} />
        </div>
      )}
    </section>
  )
}

interface SuggestionRowProps extends SectionProps {
  suggestion: AiActionSuggestion
  created: boolean
  onCreated: () => void
}

function SuggestionRow({ suggestion, recordType, recordId, created, onCreated }: SuggestionRowProps) {
  const t = useModuleTranslation(AI_KEY)
  const { hasPermission } = useSession()
  const types = useActivityTypes()
  const invalidate = useInvalidateActivitiesForRecord(recordType, recordId)
  const taskType = types.data?.find((type) => type.key === TASK_TYPE_KEY && type.is_active) ?? types.data?.find((type) => type.is_active)
  const canCreate = hasPermission('activities.create') && Boolean(taskType)

  const create = useMutation({
    mutationFn: () =>
      activitiesApi.create({
        title: suggestion.title,
        activity_type_id: taskType!.id,
        due_at: dueDate(suggestion.due_in_days),
        record_type: recordType,
        record_id: recordId,
        description: suggestion.reason || undefined,
      }),
    onSuccess: async () => {
      onCreated()
      notifySuccess(t('actions.created', { title: suggestion.title }))
      await invalidate()
    },
    onError: (error) => notifyError(error),
  })

  return (
    <li className="flex flex-col gap-2 rounded-md border px-3 py-2 sm:flex-row sm:items-center">
      <div className="grid min-w-0 flex-1 gap-0.5">
        <p className="text-sm font-medium break-words">{suggestion.title}</p>
        <p className="text-xs text-muted-foreground">
          {suggestion.due_in_days === 0 ? t('actions.dueToday') : t('actions.dueIn', { count: suggestion.due_in_days })}
          {suggestion.reason && ` · ${suggestion.reason}`}
        </p>
      </div>
      {canCreate && (
        <Button
          size="sm"
          variant={created ? 'ghost' : 'outline'}
          className="self-start sm:self-center"
          disabled={created || create.isPending}
          onClick={() => create.mutate()}
        >
          {create.isPending ? (
            <Loader2 className="animate-spin" aria-hidden="true" />
          ) : created ? (
            <Check aria-hidden="true" />
          ) : (
            <CalendarPlus aria-hidden="true" />
          )}
          {t(created ? 'actions.createdLabel' : 'actions.createTask')}
        </Button>
      )}
    </li>
  )
}
