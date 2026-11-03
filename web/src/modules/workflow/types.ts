import type { components } from '@/api'

type Schemas = components['schemas']

export type Pipeline = Schemas['PipelineResponse']
export type PipelinesPage = Schemas['Page_PipelineResponse']
export type PipelineStage = Schemas['PipelineStageResponse']
export type StagesPage = Schemas['Page_PipelineStageResponse']
export type StageKind = Schemas['StageKind']
export type LostReason = Schemas['LostReasonResponse']
export type LostReasonsPage = Schemas['Page_LostReasonResponse']
