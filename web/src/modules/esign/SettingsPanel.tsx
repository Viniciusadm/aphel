import { Copy } from 'lucide-react'
import { notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { ESIGN_KEY, useEsignStatus } from './api'

export function EsignSettingsPanel() {
  const t = useModuleTranslation(ESIGN_KEY)
  const status = useEsignStatus()

  const copy = async (value: string) => {
    try {
      await navigator.clipboard.writeText(value)
      notifySuccess(t('settingsPanel.copied'))
    } catch {
      return
    }
  }

  return (
    <div className="grid gap-3">
      <p className="text-sm text-muted-foreground">{t('legalNotice')}</p>
      <ol className="grid list-decimal gap-1 pl-5 text-sm text-muted-foreground">
        <li>{t('settingsPanel.stepToken')}</li>
        <li>{t('settingsPanel.stepWebhook')}</li>
        <li>{t('settingsPanel.stepSecret')}</li>
      </ol>
      {status.isPending ? (
        <Skeleton className="h-9 w-full" />
      ) : status.data ? (
        <div className="grid gap-1.5">
          <Label htmlFor="esign-webhook-url">{t('settingsPanel.webhookUrl')}</Label>
          <div className="flex gap-2">
            <Input placeholder={t('placeholders.webhookUrl')} id="esign-webhook-url" readOnly value={status.data.webhook_url} className="min-w-0 font-mono text-xs" />
            <Button
              type="button"
              size="icon"
              variant="outline"
              aria-label={t('settingsPanel.copy')}
              onClick={() => void copy(status.data.webhook_url)}
            >
              <Copy aria-hidden="true" />
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">
            {status.data.configured ? t('settingsPanel.configured') : t('settingsPanel.notConfigured')}
          </p>
        </div>
      ) : (
        <p className="text-xs text-muted-foreground">{t('settingsPanel.enableFirst')}</p>
      )}
    </div>
  )
}
