import { useMutation, useQueryClient } from '@tanstack/react-query'
import { ArrowDown, ArrowUp, Plus, Power, PowerOff, Star, Trash2, Users } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifySuccess } from '@/components/feedback/notify'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { WORKFLOW_RECORDS_QUERY_KEY, pipelinesApi, pipelinesKeys } from './api'
import type { Pipeline } from './types'
import { cn } from '@/lib/utils'
import { notifyPipelineError } from './errors'
import { moveItem } from './order'
import { PipelineFormDialog } from './PipelineFormDialog'
import { PipelineRemovalDialog } from './PipelineRemovalDialog'
import { TeamsField } from './TeamsField'

function useInvalidatePipelines() {
  const queryClient = useQueryClient()
  return () => {
    void queryClient.invalidateQueries({ queryKey: pipelinesKeys.list() })
    void queryClient.invalidateQueries({ queryKey: WORKFLOW_RECORDS_QUERY_KEY })
  }
}

function sameTeams(left: string[], right: string[]): boolean {
  return left.length === right.length && left.every((id) => right.includes(id))
}

function PipelineTeams({ pipeline }: { pipeline: Pipeline }) {
  const { t } = useTranslation('workflow')
  const invalidate = useInvalidatePipelines()
  const [teams, setTeams] = useState<string[]>(pipeline.team_ids)
  const save = useMutation({
    mutationFn: (team_ids: string[]) => pipelinesApi.update(pipeline.id, { team_ids }),
    onSuccess: () => {
      invalidate()
      notifySuccess(t('teams.saved'))
    },
    onError: notifyPipelineError,
  })
  const changed = !sameTeams(teams, pipeline.team_ids)

  return (
    <form
      className="grid gap-3"
      onSubmit={(event) => {
        event.preventDefault()
        if (changed) save.mutate(teams)
      }}
    >
      <TeamsField selected={teams} disabled={save.isPending} onChange={setTeams} />
      <div>
        <Button type="submit" variant="outline" size="sm" disabled={!changed || save.isPending}>
          {t('teams.save')}
        </Button>
      </div>
    </form>
  )
}

export interface PipelinesSectionProps {
  workflow: Pipeline[]
  selectedId: string | undefined
  onSelect: (id: string) => void
}

export function PipelinesSection({ workflow, selectedId, onSelect }: PipelinesSectionProps) {
  const { t } = useTranslation('workflow')
  const invalidate = useInvalidatePipelines()
  const [creating, setCreating] = useState(false)
  const [removing, setRemoving] = useState<Pipeline | null>(null)
  const selected = workflow.find((pipeline) => pipeline.id === selectedId)

  const create = useMutation({
    mutationFn: pipelinesApi.create,
    onSuccess: (pipeline) => {
      invalidate()
      setCreating(false)
      onSelect(pipeline.id)
      notifySuccess(t('create.done'))
    },
    onError: notifyPipelineError,
  })
  const reorder = useMutation({
    mutationFn: (ids: string[]) => pipelinesApi.reorder(ids),
    onSuccess: invalidate,
    onError: notifyPipelineError,
  })
  const action = useMutation({
    mutationFn: ({ pipeline, type }: { pipeline: Pipeline; type: 'default' | 'activate' | 'deactivate' }) => {
      if (type === 'default') return pipelinesApi.setDefault(pipeline.id)
      if (type === 'activate') return pipelinesApi.activate(pipeline.id)
      return pipelinesApi.deactivate(pipeline.id)
    },
    onSuccess: (_pipeline, variables) => {
      invalidate()
      notifySuccess(t(`done.${variables.type}`))
    },
    onError: notifyPipelineError,
  })
  const remove = useMutation({
    mutationFn: ({ pipeline, body }: { pipeline: Pipeline; body: Parameters<typeof pipelinesApi.remove>[1] }) =>
      pipelinesApi.remove(pipeline.id, body),
    onSuccess: () => {
      invalidate()
      setRemoving(null)
      const fallback = workflow.find((pipeline) => pipeline.is_default) ?? workflow[0]
      if (fallback) onSelect(fallback.id)
      notifySuccess(t('done.remove'))
    },
    onError: notifyPipelineError,
  })

  return (
    <Card>
      <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="grid gap-1">
          <CardTitle>{t('list.title')}</CardTitle>
          <CardDescription>{t('list.description')}</CardDescription>
        </div>
        <Button size="sm" onClick={() => setCreating(true)}>
          <Plus aria-hidden="true" />
          {t('list.new')}
        </Button>
      </CardHeader>
      <CardContent className="grid gap-4">
        {workflow.length === 0 && (
          <p className="rounded-md border border-dashed p-4 text-sm text-muted-foreground">{t('list.empty')}</p>
        )}
        <ol aria-label={t('list.title')} className="grid gap-2">
          {workflow.map((pipeline, index) => (
            <li
              key={pipeline.id}
              className={cn(
                'flex flex-wrap items-center gap-2 rounded-md border p-2',
                pipeline.id === selectedId && 'border-primary bg-primary/5',
                !pipeline.is_active && 'opacity-70',
              )}
            >
              <div className="flex flex-col">
                <Button
                  variant="ghost"
                  size="icon"
                  className="size-6"
                  disabled={index === 0 || reorder.isPending}
                  aria-label={t('list.moveUp', { name: pipeline.name })}
                  onClick={() => reorder.mutate(moveItem(workflow, index, -1).map((item) => item.id))}
                >
                  <ArrowUp aria-hidden="true" />
                </Button>
                <Button
                  variant="ghost"
                  size="icon"
                  className="size-6"
                  disabled={index === workflow.length - 1 || reorder.isPending}
                  aria-label={t('list.moveDown', { name: pipeline.name })}
                  onClick={() => reorder.mutate(moveItem(workflow, index, 1).map((item) => item.id))}
                >
                  <ArrowDown aria-hidden="true" />
                </Button>
              </div>
              <button
                type="button"
                className="grid min-w-32 flex-1 gap-0.5 text-left"
                aria-pressed={pipeline.id === selectedId}
                aria-label={t('list.select', { name: pipeline.name })}
                onClick={() => onSelect(pipeline.id)}
              >
                <span className="truncate font-medium">{pipeline.name}</span>
                <span className="text-xs text-muted-foreground">
                  {t('list.stages', { count: pipeline.stages.length })}
                </span>
              </button>
              {pipeline.is_default && <Badge variant="secondary">{t('badges.default')}</Badge>}
              {!pipeline.is_active && <Badge variant="outline">{t('badges.inactive')}</Badge>}
              {pipeline.team_ids.length > 0 && (
                <Badge variant="outline">
                  <Users aria-hidden="true" className="size-3" />
                  {t('badges.restricted', { count: pipeline.team_ids.length })}
                </Badge>
              )}
            </li>
          ))}
        </ol>
        {selected && (
          <div className="grid gap-4 rounded-md border p-3">
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="min-w-0 flex-1 truncate text-sm font-semibold">{selected.name}</h3>
              {!selected.is_default && selected.is_active && (
                <Button
                  size="sm"
                  variant="outline"
                  disabled={action.isPending}
                  onClick={() => action.mutate({ pipeline: selected, type: 'default' })}
                >
                  <Star aria-hidden="true" />
                  {t('actions.makeDefault')}
                </Button>
              )}
              {!selected.is_default && (
                <Button
                  size="sm"
                  variant="outline"
                  disabled={action.isPending}
                  onClick={() =>
                    action.mutate({ pipeline: selected, type: selected.is_active ? 'deactivate' : 'activate' })
                  }
                >
                  {selected.is_active ? <PowerOff aria-hidden="true" /> : <Power aria-hidden="true" />}
                  {selected.is_active ? t('actions.deactivate') : t('actions.activate')}
                </Button>
              )}
              {!selected.is_default && (
                <Button size="sm" variant="ghost" className="text-destructive" onClick={() => setRemoving(selected)}>
                  <Trash2 aria-hidden="true" />
                  {t('actions.remove')}
                </Button>
              )}
            </div>
            <PipelineTeams key={`${selected.id}:${selected.team_ids.join(',')}`} pipeline={selected} />
          </div>
        )}
      </CardContent>
      <PipelineFormDialog
        open={creating}
        pending={create.isPending}
        onOpenChange={setCreating}
        onSubmit={(values) => create.mutate(values)}
      />
      <PipelineRemovalDialog
        pipeline={removing}
        workflow={workflow}
        pending={remove.isPending}
        onOpenChange={(open) => {
          if (!open) setRemoving(null)
        }}
        onConfirm={(body) => removing && remove.mutate({ pipeline: removing, body })}
      />
    </Card>
  )
}
