import type { Pipeline, PipelineStage } from './types'

export function recordCount(pipeline: Pipeline): number {
  return pipeline.stages.reduce((total, stage) => total + (stage.record_count ?? 0), 0)
}

export function compatibleStages(target: Pipeline | undefined, source: PipelineStage): PipelineStage[] {
  return target?.stages.filter((stage) => stage.is_active && stage.kind === source.kind) ?? []
}

export function defaultMapping(source: Pipeline, target: Pipeline | undefined): Record<string, string> {
  const mapping: Record<string, string> = {}
  for (const stage of source.stages) {
    if ((stage.record_count ?? 0) === 0) continue
    const candidate = compatibleStages(target, stage)[0]
    if (candidate) mapping[stage.id] = candidate.id
  }
  return mapping
}
