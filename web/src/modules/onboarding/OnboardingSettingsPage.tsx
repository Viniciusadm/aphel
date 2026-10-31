import { useQueryClient } from '@tanstack/react-query'
import { RotateCcw } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'
import { notifyError } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { onboardingApi, onboardingKeys, useOnboardingState, useTemplates } from './api'
import type { OnboardingState } from './types'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function OnboardingSettingsPage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <OnboardingSettingsSection />
    </RequirePermission>
  )
}

function formatDateTime(value: string, locale: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return new Intl.DateTimeFormat(locale, { dateStyle: 'short', timeStyle: 'short' }).format(date)
}

function OnboardingSettingsSection() {
  const { t } = useTranslation('onboarding')
  const state = useOnboardingState()
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const [reopening, setReopening] = useState(false)

  const reopen = async () => {
    setReopening(true)
    try {
      const next = await onboardingApi.reopen()
      queryClient.setQueryData(onboardingKeys.state(), next)
      await navigate('/setup')
    } catch (error) {
      notifyError(error)
    } finally {
      setReopening(false)
    }
  }

  return (
    <Page>
      <PageHeader
        title={t('settings.title')}
        description={t('settings.description')}
        actions={
          <Button size="sm" onClick={() => void reopen()} disabled={reopening || state.isPending}>
            <RotateCcw aria-hidden="true" />
            {t('actions.reopen')}
          </Button>
        }
      />
      {state.isPending ? (
        <Skeleton className="h-24 w-full" />
      ) : state.isError ? (
        <ErrorState description={t('errors.load')} onRetry={() => void state.refetch()} />
      ) : (
        <StatusCard state={state.data} />
      )}
      <p className="text-sm text-muted-foreground">
        {t('settings.packagesHint')}{' '}
        <Link to="/data/config-packages" className="font-medium text-primary underline-offset-4 hover:underline">
          {t('settings.packagesLink')}
        </Link>
      </p>
    </Page>
  )
}

function StatusCard({ state }: { state: OnboardingState }) {
  const { t, i18n } = useTranslation('onboarding')
  const templates = useTemplates(state.template_key !== null)
  const templateName =
    state.template_key === null
      ? null
      : (templates.data?.find((template) => template.key === state.template_key)?.name ?? state.template_key)

  let detail: string
  if (state.status === 'completed' && state.completed_at) {
    const when = formatDateTime(state.completed_at, i18n.language)
    detail = templateName
      ? t('settings.completedWithTemplate', { template: templateName, when })
      : t('settings.completedFromScratch', { when })
  } else if (state.status === 'skipped' && state.skipped_at) {
    detail = t('settings.skippedAt', { when: formatDateTime(state.skipped_at, i18n.language) })
  } else {
    detail = t(`settings.details.${state.status}`)
  }

  return (
    <div className="grid gap-2 rounded-lg border bg-card p-4">
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-sm font-medium">{t('settings.statusLabel')}</span>
        <Badge variant={state.status === 'completed' ? 'success' : 'secondary'}>{t(`status.${state.status}`)}</Badge>
      </div>
      <p className="text-sm text-muted-foreground">{detail}</p>
    </div>
  )
}
