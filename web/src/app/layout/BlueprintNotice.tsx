import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { notifyError } from '@/components/feedback'
import { Alert } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import { useBlueprintState, useReapplyBlueprint } from '@/kernel/modules/blueprint'

export function BlueprintNotice() {
  const { t } = useTranslation('layout')
  const { hasPermission } = useSession()
  const state = useBlueprintState()
  const reapply = useReapplyBlueprint()
  const error = state.data?.configuration_error
  if (!error || !hasPermission('settings.manage')) return null
  return (
    <Alert className="mb-4 flex flex-wrap items-center justify-between gap-3">
      <span>{t('blueprint.configurationFailed', { error })}</span>
      <Button size="sm" disabled={reapply.isPending} onClick={() => reapply.mutate(undefined, { onError: (error) => notifyError(error) })}>
        {reapply.isPending ? t('blueprint.retrying') : t('blueprint.retry')}
      </Button>
    </Alert>
  )
}
