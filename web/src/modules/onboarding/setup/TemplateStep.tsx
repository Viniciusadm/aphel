import { useQueryClient } from '@tanstack/react-query'
import { ChevronDown, ChevronUp } from 'lucide-react'
import { useCallback, useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { SESSION_QUERY_KEY } from '@/app/session/session-query'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import type { SetupStepProps } from '@/kernel/modules/define'
import { SetupStepFrame } from '@/kernel/setup'
import { PLAN_OUTDATED_CODE, onboardingApi, onboardingKeys, useOnboardingState, usePlan } from '../api'
import { PreviewStep } from '../components/PreviewStep'
import { RecommendationStep } from '../components/RecommendationStep'
import { SCRATCH } from '../lib/templates'

export function TemplateStep({ onDone }: SetupStepProps) {
  const { t } = useTranslation('onboarding')
  const { t: setupT } = useTranslation('setup')
  const queryClient = useQueryClient()
  const state = useOnboardingState()
  const [selected, setSelected] = useState<string | null>(null)
  const [showPreview, setShowPreview] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const template = selected && selected !== SCRATCH ? selected : null
  const plan = usePlan(showPreview ? template : null)
  const status = state.data?.status
  const finished = useRef(false)
  const done = useCallback(() => {
    if (finished.current) return
    finished.current = true
    onDone()
  }, [onDone])

  useEffect(() => {
    if (status && status !== 'pending' && !busy) done()
  }, [busy, done, status])

  const businessType = state.data?.answers.business_type
  const description = businessType
    ? t('setup.descriptionFor', { business: setupT(`business.types.${businessType}.label`) })
    : t('setup.description')

  const complete = async (templateKey: string | null, installId: string | null) => {
    const completed = await onboardingApi.complete({ template_key: templateKey, install_id: installId })
    queryClient.setQueryData(onboardingKeys.state(), completed)
    if (installId) {
      await queryClient.invalidateQueries({
        predicate: (query) => query.queryKey[0] !== SESSION_QUERY_KEY[0] && query.queryKey[0] !== onboardingKeys.all[0],
      })
    }
  }

  const apply = async () => {
    if (busy || !selected) return
    setBusy(true)
    setError(null)
    setNotice(null)
    try {
      if (!template) {
        await complete(null, null)
        done()
        return
      }
      const current = plan.data ?? (await queryClient.fetchQuery({ queryKey: onboardingKeys.plan(template), queryFn: () => onboardingApi.plan(template) }))
      if (!current?.valid) {
        setError(t('errors.invalidTemplate'))
        return
      }
      let installId: string
      try {
        installId = (await onboardingApi.apply(template, current.fingerprint)).install.id
      } catch (cause) {
        if (isApiError(cause) && cause.code === PLAN_OUTDATED_CODE) {
          setNotice(t('preview.outdated'))
          setShowPreview(true)
          await plan.refetch()
          return
        }
        throw cause
      }
      await complete(template, installId)
      done()
    } catch {
      setError(t('errors.generic'))
    } finally {
      setBusy(false)
    }
  }

  if (state.isError) {
    return (
      <SetupStepFrame title={t('setup.title')}>
        <ErrorState description={t('errors.load')} onRetry={() => void state.refetch()} />
      </SetupStepFrame>
    )
  }

  return (
    <SetupStepFrame
      title={t('setup.title')}
      description={state.data ? description : undefined}
      error={error}
      notice={notice}
      primary={{
        label: template ? t('actions.apply') : t('actions.finishScratch'),
        onClick: () => void apply(),
        disabled: !state.data || selected === null || (showPreview && template !== null && (plan.isFetching || plan.data?.valid === false)),
        busy,
        busyLabel: template ? t('actions.applying') : undefined,
      }}
    >
      {state.isPending ? (
        <div className="grid gap-3" aria-busy="true">
          <Skeleton className="h-24 w-full" />
          <Skeleton className="h-16 w-full" />
          <Skeleton className="h-16 w-full" />
        </div>
      ) : (
        <>
          <RecommendationStep
            recommendation={state.data?.recommendation ?? null}
            selected={selected}
            onSelect={(key) => {
              setSelected(key)
              setNotice(null)
            }}
            disabled={busy}
          />
          {template && (
            <div className="grid gap-3">
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="justify-self-start"
                aria-expanded={showPreview}
                onClick={() => setShowPreview((value) => !value)}
              >
                {showPreview ? <ChevronUp aria-hidden /> : <ChevronDown aria-hidden />}
                {t('setup.preview')}
              </Button>
              {showPreview && (
                <PreviewStep
                  plan={plan.data}
                  isPending={plan.isPending}
                  isError={plan.isError}
                  onRetry={() => void plan.refetch()}
                />
              )}
            </div>
          )}
        </>
      )}
    </SetupStepFrame>
  )
}
