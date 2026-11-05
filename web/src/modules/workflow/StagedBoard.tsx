import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Skeleton } from '@/components/ui/skeleton'
import {
  KanbanBoard,
  NativeSelect,
  activePipelines,
  defaultPipeline,
  useFieldSchema,
  usePipelines,
  type TraitListViewProps,
} from '@/sdk'

function stageFieldOf(traitConfig: unknown): string {
  const mapping = traitConfig as { stage_field?: unknown } | null
  return typeof mapping?.stage_field === 'string' ? mapping.stage_field : 'stage_id'
}

export function StagedBoard({ recordType, traitConfig, search, canEdit }: TraitListViewProps) {
  const { t } = useTranslation('workflow', { keyPrefix: 'stagedBoard' })
  const schema = useFieldSchema(recordType)
  const pipelines = usePipelines()
  const [chosen, setChosen] = useState<string | null>(null)
  const active = activePipelines(pipelines.data)
  const pipeline = active.find((candidate) => candidate.id === chosen) ?? defaultPipeline(pipelines.data)
  const field = schema.data?.fields.find((candidate) => candidate.key === stageFieldOf(traitConfig))
  const columnOptions = [...(pipeline?.stages ?? [])]
    .filter((stage) => stage.is_active)
    .sort((left, right) => left.position - right.position)
    .map((stage) => ({ value: stage.id, label: stage.name }))

  if (schema.isPending || pipelines.isPending) return <Skeleton className="h-64 w-full" />
  if (!field || !pipeline) return <p className="text-sm text-muted-foreground">{t('unavailable')}</p>

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-3">
      {active.length > 1 && (
        <NativeSelect
          aria-label={t('pipeline')}
          className="w-auto max-w-56"
          value={pipeline.id}
          onChange={(event) => setChosen(event.target.value)}
        >
          {active.map((candidate) => (
            <option key={candidate.id} value={candidate.id}>
              {candidate.name}
            </option>
          ))}
        </NativeSelect>
      )}
      <KanbanBoard recordType={recordType} field={field} search={search} canEdit={canEdit} columnOptions={columnOptions} />
    </div>
  )
}
