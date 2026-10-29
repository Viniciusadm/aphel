import { BellRing, Download } from 'lucide-react'
import { useEffect, useState, useSyncExternalStore } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { canPromptInstall, promptInstall, subscribeToInstallPrompt } from '../install-store'
import { currentSubscription, disablePush, enablePush, pushSupport } from './push'

type DeviceState = 'loading' | 'enabled' | 'disabled' | 'blocked'

function permissionState(): NotificationPermission {
  return 'Notification' in window ? Notification.permission : 'default'
}

export function PushDeviceSetting() {
  const { t } = useTranslation('pwa')
  const support = pushSupport()
  const [state, setState] = useState<DeviceState>('loading')
  const [busy, setBusy] = useState(false)
  const installable = useSyncExternalStore(subscribeToInstallPrompt, canPromptInstall, () => false)

  useEffect(() => {
    if (support !== 'supported') return
    let active = true
    void currentSubscription()
      .then((subscription) => {
        if (!active) return
        if (permissionState() === 'denied') setState('blocked')
        else setState(subscription ? 'enabled' : 'disabled')
      })
      .catch(() => {
        if (active) setState('disabled')
      })
    return () => {
      active = false
    }
  }, [support])

  const enable = async () => {
    setBusy(true)
    try {
      const permission = await enablePush()
      if (permission === 'granted') {
        setState('enabled')
        notifySuccess(t('push.enabled'))
      } else {
        setState(permission === 'denied' ? 'blocked' : 'disabled')
      }
    } catch (error) {
      notifyError(error, { title: t('push.enableError') })
    } finally {
      setBusy(false)
    }
  }

  const disable = async () => {
    setBusy(true)
    try {
      await disablePush()
      setState('disabled')
    } catch (error) {
      notifyError(error, { title: t('push.disableError') })
    } finally {
      setBusy(false)
    }
  }

  const description = (() => {
    if (support !== 'supported') return t(`push.support.${support}`)
    if (state === 'blocked') return t('push.blocked')
    if (state === 'enabled') return t('push.enabledDescription')
    return t('push.disabledDescription')
  })()

  return (
    <div className="flex flex-col gap-3 rounded-lg border p-4 sm:flex-row sm:items-center">
      <BellRing className="size-5 shrink-0 text-muted-foreground" aria-hidden="true" />
      <div className="grid min-w-0 flex-1 gap-0.5">
        <p className="text-sm font-medium">{t('push.title')}</p>
        <p className="text-xs text-muted-foreground">{description}</p>
      </div>
      <div className="flex flex-wrap gap-2">
        {installable ? (
          <Button size="sm" variant="outline" onClick={() => void promptInstall()}>
            <Download />
            {t('install.action')}
          </Button>
        ) : null}
        {support === 'supported' && state === 'enabled' ? (
          <Button size="sm" variant="outline" onClick={() => void disable()} disabled={busy}>
            {t('push.disable')}
          </Button>
        ) : null}
        {support === 'supported' && state === 'disabled' ? (
          <Button size="sm" onClick={() => void enable()} disabled={busy}>
            {t('push.enable')}
          </Button>
        ) : null}
      </div>
    </div>
  )
}
