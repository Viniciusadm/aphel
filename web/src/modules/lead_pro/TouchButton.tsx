import { Flame, Loader2 } from 'lucide-react'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import type { RecordSlotProps } from '@/kernel/modules/slots'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { LEAD_PRO_KEY, useRegisterTouch } from './api'

export function TouchButton({ record, readOnly }: RecordSlotProps) {
  const t = useModuleTranslation(LEAD_PRO_KEY)
  const touch = useRegisterTouch(String(record.id))
  if (readOnly) return null
  return (
    <Button
      variant="outline"
      size="sm"
      disabled={touch.isPending}
      onClick={() =>
        touch.mutate(undefined, {
          onSuccess: (result) => notifySuccess(t('touch.done', { temperature: result.temperature ?? 0 })),
          onError: (error) => notifyError(error),
        })
      }
    >
      {touch.isPending ? <Loader2 className="size-4 animate-spin" aria-hidden="true" /> : <Flame className="size-4" aria-hidden="true" />}
      {t('touch.action')}
    </Button>
  )
}
