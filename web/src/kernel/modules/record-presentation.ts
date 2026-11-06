import type { ComponentType } from 'react'
import type { RecordPageModel } from '@/features/records/detail/use-record-page-model'

export const RECORD_PRESENTATION_CONTRACT = 'records.Presentation'
export const DEFAULT_RECORD_PRESENTATION = 'kernel'

export interface RecordPresentationProps {
  model: RecordPageModel
  defaultTab?: string
  className?: string
}

declare module './contract-components' {
  interface ContractComponentMap {
    'records.Presentation': RecordPresentationProps
  }
}

export type RecordPresentationComponent = ComponentType<RecordPresentationProps> & {
  recordTypes?: readonly string[]
}
