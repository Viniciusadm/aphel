import { registerRecordResource } from '@/features/records/resources'

export const DEAL_RECORD_TYPE = 'deals.deal'

export const pipelineResource = registerRecordResource({
  recordType: 'pipelines.pipeline',
  basePath: '/v1/pipelines',
  titleField: 'name',
})

export const pipelineStageResource = registerRecordResource({
  recordType: 'pipelines.pipeline_stage',
  basePath: '/v1/pipeline-stages',
  titleField: 'name',
})

export const lostReasonResource = registerRecordResource({
  recordType: 'pipelines.lost_reason',
  basePath: '/v1/lost-reasons',
  titleField: 'name',
})
