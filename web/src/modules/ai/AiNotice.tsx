import { AlertCircle } from 'lucide-react'
import { Link } from 'react-router'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { AI_KEY, AI_PATH, aiErrorDetail, aiErrorKey, providerLabelKey } from './api'
import { Alert } from '@/components/ui/alert'

export function AiErrorMessage({ error }: { error: unknown }) {
  const t = useModuleTranslation(AI_KEY)
  const detail = aiErrorDetail(error)
  return (
    <Alert className="flex items-start gap-2 text-destructive">
      <AlertCircle className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
      <span className="grid gap-0.5">
        <span>{t(aiErrorKey(error))}</span>
        {detail && <span className="text-xs opacity-90">{detail}</span>}
      </span>
    </Alert>
  )
}

export function AiFooter({ provider, model }: { provider: string; model: string }) {
  const t = useModuleTranslation(AI_KEY)
  return (
    <p className="text-xs text-muted-foreground">
      {t('footer.generatedBy', { model, provider: t(providerLabelKey(provider)) })}{' '}
      <Link className="underline underline-offset-2" to={AI_PATH}>
        {t('footer.whatIsSent')}
      </Link>
    </p>
  )
}
