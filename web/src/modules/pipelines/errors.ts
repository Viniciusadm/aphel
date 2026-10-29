import { isApiError } from '@/api'
import { notifyError } from '@/components/feedback/notify'
import { toast } from '@/components/ui/toast-store'
import i18n from '@/i18n'

const DEAL_CODE_MESSAGES = {
  stage_has_deals: 'editor.settings.errors.stageHasDeals',
  closing_stages_required: 'editor.settings.errors.closingStagesRequired',
  last_lost_reason: 'editor.settings.errors.lastLostReason',
} as const

const PIPELINE_CODE_MESSAGES = {
  default_pipeline_required: 'errors.defaultPipelineRequired',
  pipeline_has_deals: 'errors.pipelineHasDeals',
  pipeline_limit_reached: 'errors.pipelineLimitReached',
} as const

function isDealCode(code: string): code is keyof typeof DEAL_CODE_MESSAGES {
  return code in DEAL_CODE_MESSAGES
}

function isPipelineCode(code: string): code is keyof typeof PIPELINE_CODE_MESSAGES {
  return code in PIPELINE_CODE_MESSAGES
}

function describe(code: string): string | null {
  if (isDealCode(code)) return i18n.t(DEAL_CODE_MESSAGES[code], { ns: 'pipelines' })
  if (isPipelineCode(code)) return i18n.t(PIPELINE_CODE_MESSAGES[code], { ns: 'pipelines' })
  return null
}

export function notifyPipelineError(error: unknown) {
  const description = isApiError(error) ? describe(error.code) : null
  if (description) {
    toast({
      title: i18n.t('editor.settings.errors.title', { ns: 'pipelines' }),
      description,
      variant: 'destructive',
    })
    return
  }
  notifyError(error)
}
