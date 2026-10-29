import { useTranslation } from 'react-i18next'
import type { SequenceStep } from '../types'

export function useStepLabel() {
  const { t } = useTranslation('sequences')
  return (step: Pick<SequenceStep, 'kind' | 'wait_days' | 'subject' | 'title'>) => {
    if (step.kind === 'wait') return t('detail.waitSummary', { count: step.wait_days })
    const detail = step.kind === 'auto_email' || step.kind === 'manual_email' ? step.subject || step.title : step.title
    return detail ? `${t(`stepKinds.${step.kind}`)}: ${detail}` : t(`stepKinds.${step.kind}`)
  }
}
