import type { Pipeline } from './types'

export interface PipelineSettingsSlotProps {
  pipeline: Pipeline
  pipelines: Pipeline[]
}

declare module '@/kernel/modules/slots' {
  interface SlotPropsMap {
    'pipelines.settings.sections': PipelineSettingsSlotProps
  }
}
