import { isApiError } from '@/api'
import { notifyError } from '@/components/feedback/notify'
import { toast } from '@/components/ui/toast-store'
import i18n from '@/i18n'

const WORKFLOW_CODE_MESSAGES = {
  stage_has_records: 'editor.settings.errors.stageHasRecords',
  closing_stages_required: 'editor.settings.errors.closingStagesRequired',
  last_lost_reason: 'editor.settings.errors.lastLostReason',
} as const

const PIPELINE_CODE_MESSAGES = {
  default_pipeline_required: 'errors.defaultPipelineRequired',
  pipeline_has_records: 'errors.pipelineHasRecords',
  pipeline_limit_reached: 'errors.pipelineLimitReached',
} as const

function isWorkflowCode(code: string): code is keyof typeof WORKFLOW_CODE_MESSAGES {
  return code in WORKFLOW_CODE_MESSAGES
}

function isPipelineCode(code: string): code is keyof typeof PIPELINE_CODE_MESSAGES {
  return code in PIPELINE_CODE_MESSAGES
}

function describe(code: string): string | null {
  if (isWorkflowCode(code)) return i18n.t(WORKFLOW_CODE_MESSAGES[code], { ns: 'workflow' })
  if (isPipelineCode(code)) return i18n.t(PIPELINE_CODE_MESSAGES[code], { ns: 'workflow' })
  return null
}

export function notifyPipelineError(error: unknown) {
  const description = isApiError(error) ? describe(error.code) : null
  if (description) {
    toast({
      title: i18n.t('editor.settings.errors.title', { ns: 'workflow' }),
      description,
      variant: 'destructive',
    })
    return
  }
  notifyError(error)
}
