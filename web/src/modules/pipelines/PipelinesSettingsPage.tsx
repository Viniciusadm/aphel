import { useMutation, useQueryClient } from '@tanstack/react-query'
import { ArrowDown, ArrowUp, Pencil, Plus, Power, PowerOff, Trash2 } from 'lucide-react'
import { Fragment, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { notifySuccess } from '@/components/feedback/notify'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import {
  DEAL_BOARDS_QUERY_KEY,
  defaultPipeline,
  lostReasonsApi,
  pipelinesApi,
  pipelinesKeys,
  useLostReasons,
  usePipelines,
} from './api'
import type { LostReason, Pipeline, PipelineStage } from './types'
import { NoAccess } from '@/features/settings/components/NoAccess'
import { Slot } from '@/kernel/modules/Slot'
import { cn } from '@/lib/utils'
import { notifyPipelineError } from './errors'
import { PipelinesSection } from './PipelinesSection'
import { moveItem } from './order'
import { RottingStageBadge } from './rotting/RottingStageBadge'
import { StageDialog, type StageFormValues } from './StageDialog'
import { StageRemovalDialog, type StageRemovalMode } from './StageRemovalDialog'
import { Field } from '@/components/ui/field'

const KIND_VARIANT = { open: 'secondary', won: 'success', lost: 'destructive' } as const

function useInvalidate() {
  const queryClient = useQueryClient()
  return () => {
    void queryClient.invalidateQueries({ queryKey: pipelinesKeys.all })
    void queryClient.invalidateQueries({ queryKey: DEAL_BOARDS_QUERY_KEY })
  }
}

function PipelineName({ pipeline }: { pipeline: Pipeline }) {
  const { t } = useTranslation('pipelines', { keyPrefix: 'editor' })
  const invalidate = useInvalidate()
  const [name, setName] = useState(pipeline.name)
  const rename = useMutation({
    mutationFn: (next: string) => pipelinesApi.update(pipeline.id, { name: next }),
    onSuccess: () => {
      invalidate()
      notifySuccess(t('settings.pipeline.renamed'))
    },
    onError: notifyPipelineError,
  })
  const changed = name.trim() !== '' && name.trim() !== pipeline.name

  return (
    <form
      className="flex flex-wrap items-end gap-2"
      onSubmit={(event: FormEvent) => {
        event.preventDefault()
        if (changed) rename.mutate(name.trim())
      }}
    >
      <Field label={t('settings.pipeline.name')} className="min-w-56 flex-1">
        {(control) => <Input placeholder={t('placeholders.pipelineName')} {...control} value={name} maxLength={140} onChange={(event) => setName(event.target.value)} />}
      </Field>
      <Button type="submit" variant="outline" disabled={!changed || rename.isPending}>
        {t('actions.save')}
      </Button>
    </form>
  )
}

function StagesSection({ pipeline }: { pipeline: Pipeline }) {
  const { t } = useTranslation('pipelines', { keyPrefix: 'editor' })
  const { t: tp } = useTranslation('pipelines')
  const invalidate = useInvalidate()
  const [editing, setEditing] = useState<PipelineStage | 'new' | null>(null)
  const [removal, setRemoval] = useState<{ stage: PipelineStage; mode: StageRemovalMode } | null>(null)
  const stages = pipeline.stages

  const save = useMutation({
    mutationFn: async (values: StageFormValues) => {
      if (editing && editing !== 'new') {
        const body: Partial<StageFormValues> = {
          name: values.name,
          probability: values.probability,
          rotting_days: values.rotting_days,
          entry_required_fields: values.entry_required_fields,
          exit_required_fields: values.exit_required_fields,
        }
        if (values.kind !== editing.kind) body.kind = values.kind
        return pipelinesApi.updateStage(editing.id, body)
      }
      return pipelinesApi.createStage(pipeline.id, values)
    },
    onSuccess: () => {
      invalidate()
      notifySuccess(t(editing === 'new' ? 'settings.stages.created' : 'settings.stages.updated'))
      setEditing(null)
    },
    onError: notifyPipelineError,
  })

  const reorder = useMutation({
    mutationFn: (ids: string[]) => pipelinesApi.reorderStages(pipeline.id, ids),
    onSuccess: invalidate,
    onError: notifyPipelineError,
  })

  const activate = useMutation({
    mutationFn: (stage: PipelineStage) => pipelinesApi.activateStage(stage.id),
    onSuccess: () => {
      invalidate()
      notifySuccess(t('settings.stages.activated'))
    },
    onError: notifyPipelineError,
  })

  const remove = useMutation({
    mutationFn: async ({ stage, mode, target }: { stage: PipelineStage; mode: StageRemovalMode; target?: string }) => {
      if (mode === 'delete') await pipelinesApi.deleteStage(stage.id, target)
      else await pipelinesApi.deactivateStage(stage.id, target)
    },
    onSuccess: (_result, variables) => {
      invalidate()
      notifySuccess(t(variables.mode === 'delete' ? 'settings.stages.deleted' : 'settings.stages.deactivated'))
      setRemoval(null)
    },
    onError: notifyPipelineError,
  })

  const move = (index: number, delta: -1 | 1) => reorder.mutate(moveItem(stages, index, delta).map((stage) => stage.id))

  return (
    <Card>
      <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="grid gap-1">
          <CardTitle>{t('settings.stages.title')}</CardTitle>
          <CardDescription>{t('settings.stages.description')}</CardDescription>
        </div>
        <Button size="sm" onClick={() => setEditing('new')}>
          <Plus aria-hidden="true" />
          {t('settings.stages.new')}
        </Button>
      </CardHeader>
      <CardContent className="grid gap-4">
        <PipelineName key={pipeline.name} pipeline={pipeline} />
        <ol aria-label={t('settings.stages.title')} className="grid gap-2">
          {stages.map((stage, index) => (
            <li
              key={stage.id}
              className={cn(
                'flex flex-wrap items-center gap-2 rounded-md border p-2',
                !stage.is_active && 'opacity-60',
              )}
            >
              <div className="flex flex-col">
                <Button
                  variant="ghost"
                  size="icon"
                  className="size-6"
                  disabled={index === 0 || reorder.isPending}
                  aria-label={t('settings.moveUp', { name: stage.name })}
                  onClick={() => move(index, -1)}
                >
                  <ArrowUp aria-hidden="true" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  className="size-6"
                  disabled={index === stages.length - 1 || reorder.isPending}
                  aria-label={t('settings.moveDown', { name: stage.name })}
                  onClick={() => move(index, 1)}
                >
                  <ArrowDown aria-hidden="true" />
                </Button>
              </div>
              <div className="grid min-w-32 flex-1 gap-0.5">
                <span className="truncate font-medium">{stage.name}</span>
                <span className="text-xs text-muted-foreground">
                  {t('settings.stages.summary', { probability: stage.probability, count: stage.deal_count ?? 0 })}
                </span>
              </div>
              <Badge variant={KIND_VARIANT[stage.kind]}>{t(`kind.${stage.kind}`)}</Badge>
              {(stage.entry_required_fields.length > 0 || stage.exit_required_fields.length > 0) && (
                <Badge variant="outline">
                  {tp('rules.badge', {
                    count: stage.entry_required_fields.length + stage.exit_required_fields.length,
                  })}
                </Badge>
              )}
              <RottingStageBadge stage={stage} />
              {!stage.is_active && <Badge variant="outline">{t('settings.inactive')}</Badge>}
              <div className="flex items-center gap-1">
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label={t('settings.stages.edit', { name: stage.name })}
                  onClick={() => setEditing(stage)}
                >
                  <Pencil aria-hidden="true" />
                </Button>
                {stage.is_active ? (
                  <Button
                    variant="ghost"
                    size="icon"
                    aria-label={t('settings.stages.deactivate', { name: stage.name })}
                    onClick={() => setRemoval({ stage, mode: 'deactivate' })}
                  >
                    <PowerOff aria-hidden="true" />
                  </Button>
                ) : (
                  <Button
                    variant="ghost"
                    size="icon"
                    aria-label={t('settings.stages.activate', { name: stage.name })}
                    disabled={activate.isPending}
                    onClick={() => activate.mutate(stage)}
                  >
                    <Power aria-hidden="true" />
                  </Button>
                )}
                <Button
                  variant="ghost"
                  size="icon"
                  className="text-destructive"
                  aria-label={t('settings.stages.delete', { name: stage.name })}
                  onClick={() => setRemoval({ stage, mode: 'delete' })}
                >
                  <Trash2 aria-hidden="true" />
                </Button>
              </div>
            </li>
          ))}
        </ol>
      </CardContent>
      <StageDialog
        open={editing !== null}
        stage={editing && editing !== 'new' ? editing : undefined}
        pending={save.isPending}
        onOpenChange={(open) => {
          if (!open) setEditing(null)
        }}
        onSubmit={(values) => save.mutate(values)}
      />
      <StageRemovalDialog
        mode={removal?.mode ?? 'deactivate'}
        stage={removal?.stage ?? null}
        stages={stages}
        pending={remove.isPending}
        onOpenChange={(open) => {
          if (!open) setRemoval(null)
        }}
        onConfirm={(target) => removal && remove.mutate({ ...removal, target })}
      />
    </Card>
  )
}

function LostReasonRow({
  reason,
  index,
  total,
  busy,
  onMove,
}: {
  reason: LostReason
  index: number
  total: number
  busy: boolean
  onMove: (index: number, delta: -1 | 1) => void
}) {
  const { t } = useTranslation('pipelines', { keyPrefix: 'editor' })
  const invalidate = useInvalidate()
  const [editing, setEditing] = useState(false)
  const [name, setName] = useState(reason.name)
  const update = useMutation({
    mutationFn: (body: { name?: string; is_active?: boolean }) => lostReasonsApi.update(reason.id, body),
    onSuccess: () => {
      invalidate()
      setEditing(false)
    },
    onError: notifyPipelineError,
  })

  return (
    <li className={cn('flex flex-wrap items-center gap-2 rounded-md border p-2', !reason.is_active && 'opacity-60')}>
      <div className="flex flex-col">
        <Button
          variant="ghost"
          size="icon"
          className="size-6"
          disabled={index === 0 || busy}
          aria-label={t('settings.moveUp', { name: reason.name })}
          onClick={() => onMove(index, -1)}
        >
          <ArrowUp aria-hidden="true" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="size-6"
          disabled={index === total - 1 || busy}
          aria-label={t('settings.moveDown', { name: reason.name })}
          onClick={() => onMove(index, 1)}
        >
          <ArrowDown aria-hidden="true" />
        </Button>
      </div>
      {editing ? (
        <form
          className="flex min-w-0 flex-1 items-center gap-2"
          onSubmit={(event) => {
            event.preventDefault()
            if (name.trim()) update.mutate({ name: name.trim() })
          }}
        >
          <Input placeholder={t('placeholders.lostReason')}
            aria-label={t('settings.lostReasons.name')}
            value={name}
            maxLength={140}
            autoFocus
            onChange={(event) => setName(event.target.value)}
          />
          <Button type="submit" size="sm" disabled={!name.trim() || update.isPending}>
            {t('actions.save')}
          </Button>
          <Button
            type="button"
            size="sm"
            variant="ghost"
            onClick={() => {
              setName(reason.name)
              setEditing(false)
            }}
          >
            {t('actions.cancel')}
          </Button>
        </form>
      ) : (
        <span className="min-w-32 flex-1 truncate font-medium">{reason.name}</span>
      )}
      {!reason.is_active && <Badge variant="outline">{t('settings.inactive')}</Badge>}
      {!editing && (
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="icon"
            aria-label={t('settings.lostReasons.edit', { name: reason.name })}
            onClick={() => setEditing(true)}
          >
            <Pencil aria-hidden="true" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            disabled={update.isPending}
            aria-label={t(reason.is_active ? 'settings.lostReasons.deactivate' : 'settings.lostReasons.activate', {
              name: reason.name,
            })}
            onClick={() => update.mutate({ is_active: !reason.is_active })}
          >
            {reason.is_active ? <PowerOff aria-hidden="true" /> : <Power aria-hidden="true" />}
          </Button>
        </div>
      )}
    </li>
  )
}

function LostReasonsSection() {
  const { t } = useTranslation('pipelines', { keyPrefix: 'editor' })
  const invalidate = useInvalidate()
  const reasons = useLostReasons(true)
  const [name, setName] = useState('')
  const create = useMutation({
    mutationFn: (next: string) => lostReasonsApi.create(next),
    onSuccess: () => {
      invalidate()
      setName('')
      notifySuccess(t('settings.lostReasons.created'))
    },
    onError: notifyPipelineError,
  })
  const reorder = useMutation({
    mutationFn: (ids: string[]) => lostReasonsApi.reorder(ids),
    onSuccess: invalidate,
    onError: notifyPipelineError,
  })
  const items = reasons.data ?? []

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('settings.lostReasons.title')}</CardTitle>
        <CardDescription>{t('settings.lostReasons.description')}</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-4">
        <form
          className="flex flex-wrap items-end gap-2"
          onSubmit={(event) => {
            event.preventDefault()
            if (name.trim()) create.mutate(name.trim())
          }}
        >
          <Field label={t('settings.lostReasons.new')} className="min-w-56 flex-1">
            {(control) => <Input placeholder={t('placeholders.lostReason')} {...control} value={name} maxLength={140} onChange={(event) => setName(event.target.value)} />}
          </Field>
          <Button type="submit" disabled={!name.trim() || create.isPending}>
            <Plus aria-hidden="true" />
            {t('settings.lostReasons.add')}
          </Button>
        </form>
        {reasons.isPending ? (
          <Skeleton className="h-32 w-full" />
        ) : reasons.isError ? (
          <ErrorState onRetry={() => void reasons.refetch()} />
        ) : (
          <ol aria-label={t('settings.lostReasons.title')} className="grid gap-2">
            {items.map((reason, index) => (
              <LostReasonRow
                key={`${reason.id}:${reason.name}`}
                reason={reason}
                index={index}
                total={items.length}
                busy={reorder.isPending}
                onMove={(from, delta) => reorder.mutate(moveItem(items, from, delta).map((item) => item.id))}
              />
            ))}
          </ol>
        )}
      </CardContent>
    </Card>
  )
}

export function PipelinesSettingsPage() {
  const { t } = useTranslation('pipelines')
  const { hasPermission } = useSession()
  const pipelines = usePipelines()
  const [selectedId, setSelectedId] = useState<string | undefined>()

  if (!hasPermission('settings.manage')) return <NoAccess />

  const pipeline = pipelines.data?.find((item) => item.id === selectedId) ?? defaultPipeline(pipelines.data)

  return (
    <div className="grid gap-6">
      <div className="grid gap-1">
        <h2 className="text-lg font-semibold">{t('settings.title')}</h2>
        <p className="text-sm text-muted-foreground">{t('settings.description')}</p>
      </div>
      {pipelines.isPending ? (
        <Skeleton className="h-64 w-full" />
      ) : pipelines.isError || !pipeline ? (
        <ErrorState onRetry={() => void pipelines.refetch()} />
      ) : (
        <>
          <PipelinesSection pipelines={pipelines.data} selectedId={pipeline.id} onSelect={setSelectedId} />
          <StagesSection key={pipeline.id} pipeline={pipeline} />
          <Fragment key={pipeline.id}>
            <Slot name="pipelines.settings.sections" props={{ pipeline, pipelines: pipelines.data }} />
          </Fragment>
        </>
      )}
      <LostReasonsSection />
    </div>
  )
}
