import { Check, Copy } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { HOOKS_PATH, hookUrl, webhookActions, type IssuedIncomingWebhook } from './api'
import { curlExample, samplePayload, toFormValues } from './form'

function CopyField({ label, value, testId }: { label: string; value: string; testId: string }) {
  const { t } = useTranslation('webhooks', { keyPrefix: 'incoming' })
  const [copied, setCopied] = useState(false)

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(value)
      setCopied(true)
    } catch {
      notifyError(new Error(t('credentials.copyFailed')))
    }
  }

  return (
    <div className="grid gap-1.5">
      <span className="text-sm font-medium">{label}</span>
      <code data-testid={testId} className="block rounded-md bg-muted p-3 text-xs break-all">
        {value}
      </code>
      <Button type="button" variant="outline" size="sm" className="self-start" onClick={() => void copy()}>
        {copied ? <Check aria-hidden="true" /> : <Copy aria-hidden="true" />}
        {copied ? t('credentials.copied') : t('credentials.copy')}
      </Button>
    </div>
  )
}

interface CredentialsDialogProps {
  issued: IssuedIncomingWebhook | null
  onClose: () => void
}

export function CredentialsDialog({ issued, onClose }: CredentialsDialogProps) {
  const { t } = useTranslation('webhooks', { keyPrefix: 'incoming' })
  const url = issued?.path ? hookUrl(issued.path) : null
  const exampleUrl = url ?? (issued ? hookUrl(`${HOOKS_PATH}${issued.token_prefix}…`) : '')
  const actions = issued ? toFormValues(issued).actions : []
  const example = issued ? curlExample(exampleUrl, samplePayload(actions), issued.secret_configured) : null
  const showsCredentials = Boolean(url || issued?.signing_secret)

  return (
    <Dialog open={issued !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            {showsCredentials
              ? t('credentials.title', { name: issued?.name ?? '' })
              : t('credentials.exampleTitle', { name: issued?.name ?? '' })}
          </DialogTitle>
          <DialogDescription>
            {showsCredentials ? t('credentials.description') : t('credentials.exampleDescription')}
          </DialogDescription>
        </DialogHeader>
        {issued && (
          <div className="grid gap-4">
            {url && <CopyField label={t('credentials.url')} value={url} testId="incoming-webhook-url" />}
            {issued.signing_secret && (
              <CopyField
                label={t('credentials.secret')}
                value={issued.signing_secret}
                testId="incoming-webhook-secret"
              />
            )}
            {example && (
              <div className="grid gap-1.5">
                <span className="text-sm font-medium">{t('credentials.example')}</span>
                <p className="text-xs text-muted-foreground">
                  {webhookActions(issued).length > 0 ? t('credentials.exampleHelp') : t('credentials.exampleNoActions')}
                </p>
                <pre className="overflow-x-auto rounded-md border p-3 text-xs">{example}</pre>
              </div>
            )}
          </div>
        )}
        <DialogFooter>
          <Button type="button" onClick={onClose}>
            {t('credentials.done')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
