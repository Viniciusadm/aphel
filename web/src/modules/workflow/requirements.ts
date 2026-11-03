import type { PipelineStage } from './types'

export function requiredStageFields(from: PipelineStage | undefined, to: PipelineStage): string[] {
  const keys: string[] = []
  for (const key of [...(from?.exit_required_fields ?? []), ...(to.entry_required_fields ?? [])]) {
    if (!keys.includes(key)) keys.push(key)
  }
  return keys
}
