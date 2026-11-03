import type { Pipeline } from './types'

export interface PipelineSettingsSlotProps {
  pipeline: Pipeline
  workflow: Pipeline[]
}

declare module '@/kernel/modules/slots' {
  interface SlotPropsMap {
    'workflow.settings.sections': PipelineSettingsSlotProps
  }
}
