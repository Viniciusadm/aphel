import { useTranslation } from 'react-i18next'
import { notifySuccess } from '@/components/feedback/notify'
import type { OutboxResult } from '@/pwa/offline/outbox'

export function useOutboxFeedback() {
  const { t } = useTranslation('mobile')
  return <T>(result: OutboxResult<T>, sentMessage: string) => {
    if (result.status === 'queued') notifySuccess(t('quickAdd.queued'))
    else notifySuccess(sentMessage)
  }
}
