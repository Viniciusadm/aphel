import { registerRecordResource } from '@/features/records/resources'

export const DEAL_RECORD_TYPE = 'deal'

export const pipelineResource = registerRecordResource({
  recordType: 'pipeline',
  basePath: '/v1/pipelines',
  titleField: 'name',
})

export const pipelineStageResource = registerRecordResource({
  recordType: 'pipeline_stage',
  basePath: '/v1/pipeline-stages',
  titleField: 'name',
})

export const lostReasonResource = registerRecordResource({
  recordType: 'lost_reason',
  basePath: '/v1/lost-reasons',
  titleField: 'name',
})
