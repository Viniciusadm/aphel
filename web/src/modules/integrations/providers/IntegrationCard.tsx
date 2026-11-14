import { ExternalLink, Mail, Plug, Webhook, type LucideIcon } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { isApiError } from '@/api'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
import { Badge, type BadgeProps } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import { ToggleField } from '@/components/ui/toggle-field'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { toast } from '@/components/ui/toast-store'
import { formatDateTime, useRecordFormatting } from '@/sdk'
import {
  TEST_EMAIL_PROVIDERS,
  integrationStatus,
  integrationsApi,
  type Integration,
  type IntegrationStatus,
} from './api'
import { IntegrationConfigDialog } from './IntegrationConfigDialog'
import { useIntegrationText } from './text'
import { Alert } from '@/components/ui/alert'

const ICONS: Record<string, LucideIcon> = {
  mail: Mail,
  webhook: Webhook,
}

const STATUS_VARIANTS: Record<IntegrationStatus, BadgeProps['variant']> = {
  not_configured: 'outline',
  inactive: 'secondary',
  active: 'success',
  error: 'destructive',
  available: 'secondary',
}

type ErrorSource = 'test' | 'activation' | 'delivery'

interface IntegrationCardProps {
  integration: Integration
  onChanged: () => Promise<unknown>
}

export function IntegrationCard({ integration, onChanged }: IntegrationCardProps) {
  const { t } = useTranslation('integration-card')
  const text = useIntegrationText()
  const formatting = useRecordFormatting()
  const status = integrationStatus(integration)
  const Icon = ICONS[integration.icon] ?? Plug
  const name = text.name(integration)
  const [configuring, setConfiguring] = useState(false)
  const [testing, setTesting] = useState(false)
  const [confirm, setConfirm] = useState<'remove' | 'deactivate' | null>(null)
  const [busy, setBusy] = useState(false)
  const shortcut = integration.kind === 'shortcut'
  const enabled = status === 'active' || status === 'error'

  const activate = async () => {
    setBusy(true)
    try {
      await integrationsApi.activate(integration.provider_key)
      notifySuccess(t('toast.activated', { name }))
    } catch (error) {
      if (isApiError(error) && ['integration_test_failed', 'integration_activation_failed'].includes(error.code)) {
        toast({ title: t('toast.activationFailed', { name }), description: error.message, variant: 'destructive' })
      } else if (isApiError(error) && error.code === 'integration_not_configured') {
        toast({ title: t('toast.activationFailed', { name }), description: t('errors.notConfigured'), variant: 'destructive' })
      } else {
        notifyError(error)
      }
    } finally {
      setBusy(false)
      await onChanged()
    }
  }

  const deactivate = async () => {
    await integrationsApi.deactivate(integration.provider_key)
    notifySuccess(t('toast.deactivated', { name }))
    await onChanged()
  }

  const remove = async () => {
    await integrationsApi.remove(integration.provider_key)
    notifySuccess(t('toast.removed', { name }))
    await onChanged()
  }

  return (
    <li className="contents">
      <Card className="gap-4 py-4">
        <CardHeader className="px-4">
          <div className="flex items-start justify-between gap-3">
            <div className="flex min-w-0 items-center gap-3">
              <span className="flex size-9 shrink-0 items-center justify-center rounded-md bg-muted">
                <Icon className="size-5" aria-hidden="true" />
              </span>
              <CardTitle className="break-words">{name}</CardTitle>
            </div>
            <Badge variant={STATUS_VARIANTS[status]}>{t(`status.${status}`)}</Badge>
          </div>
          <CardDescription>{text.description(integration)}</CardDescription>
        </CardHeader>
        {!shortcut && integration.configured && (
          <CardContent className="grid gap-1 px-4 text-xs text-muted-foreground">
            <p>
              {integration.last_tested_at
                ? t(integration.last_test_ok ? 'lastTestOk' : 'lastTestFailed', {
                    date: formatDateTime(integration.last_tested_at, formatting),
                  })
                : t('neverTested')}
            </p>
            {integration.last_error && integration.last_error_at && (
              <Alert className="text-destructive">
                <p className="font-medium">{t('lastError', { date: formatDateTime(integration.last_error_at, formatting) })}</p>
                <p className="break-words">{integration.last_error}</p>
              </Alert>
            )}
            {integration.recent_errors.length > 1 && (
              <details>
                <summary className="cursor-pointer">{t('recentErrors', { count: integration.recent_errors.length })}</summary>
                <ul className="mt-1 grid gap-1">
                  {integration.recent_errors.map((error) => (
                    <li key={`${error.at}-${error.source}`} className="break-words">
                      <span className="font-medium">
                        {formatDateTime(error.at, formatting)} · {t(`errorSource.${error.source as ErrorSource}`)}
                      </span>
                      {': '}
                      {error.message}
                    </li>
                  ))}
                </ul>
              </details>
            )}
          </CardContent>
        )}
        <CardFooter className="mt-auto flex-wrap gap-2 px-4">
          {shortcut && integration.path ? (
            <Button asChild size="sm" variant="outline">
              <Link to={integration.path}>
                <ExternalLink aria-hidden="true" />
                {t('actions.open')}
              </Link>
            </Button>
          ) : (
            <>
              <Button size="sm" variant={integration.configured ? 'outline' : 'default'} onClick={() => setConfiguring(true)}>
                {integration.configured ? t('actions.edit') : t('actions.configure')}
              </Button>
              {integration.configured && (
                <Button size="sm" variant="outline" onClick={() => setTesting(true)}>
                  {t('actions.test')}
                </Button>
              )}
              {integration.configured &&
                (enabled ? (
                  <Button size="sm" variant="outline" onClick={() => setConfirm('deactivate')}>
                    {t('actions.deactivate')}
                  </Button>
                ) : (
                  <Button size="sm" disabled={busy} onClick={() => void activate()}>
                    {t('actions.activate')}
                  </Button>
                ))}
              {integration.configured && (
                <Button size="sm" variant="ghost" onClick={() => setConfirm('remove')}>
                  {t('actions.remove')}
                </Button>
              )}
            </>
          )}
        </CardFooter>
      </Card>
      {!shortcut && (
        <IntegrationConfigDialog
          key={integration.updated_at ?? 'new'}
          integration={integration}
          open={configuring}
          onOpenChange={setConfiguring}
          onSaved={() => void onChanged()}
        />
      )}
      {testing && (
        <TestConnectionDialog
          integration={integration}
          name={name}
          onClose={() => setTesting(false)}
          onTested={onChanged}
        />
      )}
      <ConfirmDialog
        open={confirm !== null}
        onOpenChange={(open) => !open && setConfirm(null)}
        title={confirm === 'remove' ? t('removeTitle') : t('deactivateTitle')}
        description={confirm === 'remove' ? t('removeDescription', { name }) : t('deactivateDescription', { name })}
        confirmLabel={confirm === 'remove' ? t('removeConfirm') : t('deactivateConfirm')}
        variant="destructive"
        onConfirm={() => (confirm === 'remove' ? remove() : deactivate())}
      />
    </li>
  )
}

interface TestConnectionDialogProps {
  integration: Integration
  name: string
  onClose: () => void
  onTested: () => Promise<unknown>
}

function TestConnectionDialog({ integration, name, onClose, onTested }: TestConnectionDialogProps) {
  const { t } = useTranslation('integration-card')
  const checkboxId = useId()
  const canSendEmail = TEST_EMAIL_PROVIDERS.includes(integration.provider_key)
  const [sendEmail, setSendEmail] = useState(false)
  const [running, setRunning] = useState(false)

  const run = async () => {
    setRunning(true)
    try {
      const result = await integrationsApi.test(integration.provider_key, canSendEmail && sendEmail)
      if (result.ok) {
        notifySuccess(result.sent_to ? t('toast.testOkEmail', { email: result.sent_to }) : t('toast.testOk', { name }))
      } else {
        toast({ title: t('toast.testFailed'), description: result.message ?? undefined, variant: 'destructive' })
      }
      onClose()
    } catch (error) {
      if (isApiError(error) && error.code === 'integration_not_configured') {
        toast({ title: t('toast.testFailed'), description: t('errors.notConfigured'), variant: 'destructive' })
      } else {
        notifyError(error)
      }
    } finally {
      setRunning(false)
      await onTested()
    }
  }

  return (
    <Dialog open onOpenChange={(open) => !open && !running && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('test.title')}</DialogTitle>
          <DialogDescription>{t('test.description')}</DialogDescription>
        </DialogHeader>
        {canSendEmail && (
          <ToggleField id={checkboxId} label={t('test.sendEmail')} checked={sendEmail} onCheckedChange={setSendEmail} />
        )}
        <DialogFooter>
          <Button type="button" variant="outline" disabled={running} onClick={onClose}>
            {t('test.cancel')}
          </Button>
          <Button type="button" disabled={running} onClick={() => void run()}>
            {t('test.submit')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
