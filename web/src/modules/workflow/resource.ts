import { registerRecordResource } from '@/features/records/resources'

export const pipelineResource = registerRecordResource({
  recordType: 'workflow.pipeline',
  basePath: '/v1/workflow',
  titleField: 'name',
})

export const pipelineStageResource = registerRecordResource({
  recordType: 'workflow.stage',
  basePath: '/v1/pipeline-stages',
  titleField: 'name',
})

export const lostReasonResource = registerRecordResource({
  recordType: 'workflow.lost_reason',
  basePath: '/v1/lost-reasons',
  titleField: 'name',
})
