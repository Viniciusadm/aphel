import { useQueryClient } from '@tanstack/react-query'
import { LoaderCircle } from 'lucide-react'
import { useEffect, useRef, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'
import { isApiError } from '@/api'
import { useSession } from '@/app/session/session-context'
import { SESSION_QUERY_KEY } from '@/app/session/session-query'
import { notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardFooter, CardHeader } from '@/components/ui/card'
import { ErrorState } from '@/components/ui/error-state'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { PLAN_OUTDATED_CODE, onboardingApi, onboardingKeys, usePlan, useOnboardingState } from './api'
import { OnboardingShell } from './components/OnboardingShell'
import { OptionCard } from './components/OptionCard'
import { PreviewStep } from './components/PreviewStep'
import { RecommendationStep } from './components/RecommendationStep'
import { BUSINESS_TYPES, OFFERINGS, SCRATCH, STEPS, TEAM_SIZES, type Step } from './lib/options'
import { markPrompted } from './lib/prompt'
import type { BusinessType, Offering, OnboardingState, TeamSize } from './types'

export function OnboardingPage() {
  const { t } = useTranslation('onboarding')
  const { hasPermission } = useSession()
  const canManage = hasPermission(SETTINGS_MANAGE)
  const state = useOnboardingState(canManage)

  let content
  if (!canManage) {
    content = (
      <MessageCard title={t('noAccess.title')} description={t('noAccess.description')}>
        <Button asChild>
          <Link to="/">{t('page.goToDashboard')}</Link>
        </Button>
      </MessageCard>
    )
  } else if (state.isPending) {
    content = (
      <div role="status" className="flex items-center justify-center gap-2 py-12 text-sm text-muted-foreground">
        <LoaderCircle className="size-5 animate-spin" aria-hidden />
        <span>{t('page.loading')}</span>
      </div>
    )
  } else if (state.isError) {
    content = <ErrorState description={t('errors.load')} onRetry={() => void state.refetch()} />
  } else if (state.data.status !== 'pending') {
    content = <NotPendingCard state={state.data} />
  } else {
    content = <OnboardingWizard state={state.data} />
  }

  return <OnboardingShell>{content}</OnboardingShell>
}

interface MessageCardProps {
  title: string
  description: string
  children?: ReactNode
}

function MessageCard({ title, description, children }: MessageCardProps) {
  return (
    <Card>
      <CardHeader>
        <h1 className="text-xl font-semibold tracking-tight">{title}</h1>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      {children && <CardFooter className="flex flex-wrap gap-2">{children}</CardFooter>}
    </Card>
  )
}

function NotPendingCard({ state }: { state: OnboardingState }) {
  const { t } = useTranslation('onboarding')
  const queryClient = useQueryClient()
  const [reopening, setReopening] = useState(false)
  const [failed, setFailed] = useState(false)

  const reopen = async () => {
    setReopening(true)
    setFailed(false)
    try {
      const next = await onboardingApi.reopen()
      queryClient.setQueryData(onboardingKeys.state(), next)
    } catch {
      setFailed(true)
    } finally {
      setReopening(false)
    }
  }

  return (
    <MessageCard title={t('page.title')} description={t(`notPending.${state.status as 'completed'}`)}>
      {failed && (
        <p role="alert" className="w-full text-sm text-destructive">
          {t('errors.generic')}
        </p>
      )}
      <Button type="button" onClick={() => void reopen()} disabled={reopening}>
        {t('actions.reopen')}
      </Button>
      <Button asChild variant="outline">
        <Link to="/">{t('page.goToDashboard')}</Link>
      </Button>
    </MessageCard>
  )
}

interface WizardProps {
  state: OnboardingState
}

function OnboardingWizard({ state }: WizardProps) {
  const { t } = useTranslation('onboarding')
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const headingRef = useRef<HTMLHeadingElement>(null)
  const [step, setStep] = useState<Step>('business')
  const [businessType, setBusinessType] = useState<BusinessType | null>(state.answers.business_type ?? null)
  const [offering, setOffering] = useState<Offering[]>(state.answers.offering)
  const [teamSize, setTeamSize] = useState<TeamSize | null>(state.answers.team_size ?? null)
  const [recommendation, setRecommendation] = useState(state.recommendation ?? null)
  const [selected, setSelected] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [installId, setInstallId] = useState<string | null>(null)
  const previewTemplate = step === 'preview' && selected && selected !== SCRATCH ? selected : null
  const plan = usePlan(previewTemplate)
  const stepIndex = STEPS.indexOf(step)

  useEffect(() => {
    headingRef.current?.focus()
  }, [step])

  const goTo = (next: Step) => {
    setError(null)
    setNotice(null)
    setStep(next)
  }

  const storeState = (next: OnboardingState) => {
    queryClient.setQueryData(onboardingKeys.state(), next)
  }

  const leave = async () => {
    markPrompted()
    await navigate('/', { replace: true })
  }

  const run = async (action: () => Promise<void>) => {
    setBusy(true)
    setError(null)
    try {
      await action()
    } catch {
      setError(t('errors.generic'))
    } finally {
      setBusy(false)
    }
  }

  const skip = () =>
    run(async () => {
      try {
        storeState(await onboardingApi.skip())
      } catch (cause) {
        if (!isApiError(cause) || cause.status !== 409) throw cause
      }
      await leave()
    })

  const saveAnswers = () =>
    run(async () => {
      const saved = await onboardingApi.saveAnswers({
        business_type: businessType,
        offering,
        team_size: teamSize,
      })
      storeState(saved)
      setRecommendation(saved.recommendation ?? null)
      setSelected(null)
      goTo('recommendation')
    })

  const finish = async (templateKey: string | null, install: string | null) => {
    const completed = await onboardingApi.complete({ template_key: templateKey, install_id: install })
    notifySuccess(templateKey ? t('done.applied') : t('done.scratch'))
    await leave()
    storeState(completed)
    if (install) {
      await queryClient.invalidateQueries({
        predicate: (query) => query.queryKey[0] !== SESSION_QUERY_KEY[0] && query.queryKey[0] !== onboardingKeys.all[0],
      })
    }
  }

  const complete = (templateKey: string | null, install: string | null) => run(() => finish(templateKey, install))

  const apply = (templateKey: string) =>
    run(async () => {
      const fingerprint = plan.data?.fingerprint
      if (!fingerprint) return
      let install: string
      try {
        install = (await onboardingApi.apply(templateKey, fingerprint)).install.id
      } catch (cause) {
        if (isApiError(cause) && cause.code === PLAN_OUTDATED_CODE) {
          setNotice(t('preview.outdated'))
          await plan.refetch()
          return
        }
        throw cause
      }
      setInstallId(install)
      await finish(templateKey, install)
    })

  const chosen = selected

  const canContinue =
    (step === 'business' && businessType !== null) ||
    (step === 'offering' && offering.length > 0) ||
    (step === 'team' && teamSize !== null) ||
    (step === 'recommendation' && selected !== null)

  const onNext = () => {
    if (step === 'business') goTo('offering')
    else if (step === 'offering') goTo('team')
    else if (step === 'team') void saveAnswers()
    else if (step === 'recommendation') {
      if (chosen === SCRATCH) void complete(null, null)
      else goTo('preview')
    }
  }

  const onBack = () => {
    if (stepIndex > 0) goTo(STEPS[stepIndex - 1])
  }

  const toggleOffering = (value: Offering, checked: boolean) => {
    setOffering((current) =>
      checked ? [...new Set([...current, value])] : current.filter((item) => item !== value),
    )
  }

  return (
    <Card>
      <CardHeader className="gap-3">
        <div className="grid gap-2">
          <p className="text-xs font-medium text-muted-foreground">
            {t('page.stepLabel', { current: stepIndex + 1, total: STEPS.length })}
          </p>
          <div
            role="progressbar"
            aria-label={t('page.progressLabel')}
            aria-valuemin={1}
            aria-valuemax={STEPS.length}
            aria-valuenow={stepIndex + 1}
            className="h-1.5 w-full overflow-hidden rounded-full bg-muted"
          >
            <div
              className="h-full rounded-full bg-primary transition-all"
              style={{ width: `${((stepIndex + 1) / STEPS.length) * 100}%` }}
            />
          </div>
        </div>
        <h1 ref={headingRef} tabIndex={-1} className="text-xl font-semibold tracking-tight outline-none">
          {t(`steps.${step}.title`)}
        </h1>
        <CardDescription>{t(`steps.${step}.description`)}</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-4">
        {notice && (
          <p role="status" className="rounded-md border border-warning/40 bg-warning/10 p-3 text-sm">
            {notice}
          </p>
        )}
        {error && (
          <p role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">
            {error}
          </p>
        )}
        {step === 'business' && (
          <fieldset className="grid gap-3 sm:grid-cols-2" disabled={busy}>
            <legend className="sr-only">{t('steps.business.title')}</legend>
            {BUSINESS_TYPES.map((value) => (
              <OptionCard
                key={value}
                type="radio"
                name="onboarding-business-type"
                value={value}
                checked={businessType === value}
                onChange={() => setBusinessType(value)}
                title={t(`businessTypes.${value}.label`)}
                description={t(`businessTypes.${value}.description`)}
              />
            ))}
          </fieldset>
        )}
        {step === 'offering' && (
          <fieldset className="grid gap-3" disabled={busy}>
            <legend className="sr-only">{t('steps.offering.title')}</legend>
            {OFFERINGS.map((value) => (
              <OptionCard
                key={value}
                type="checkbox"
                name="onboarding-offering"
                value={value}
                checked={offering.includes(value)}
                onChange={(checked) => toggleOffering(value, checked)}
                title={t(`offerings.${value}.label`)}
                description={t(`offerings.${value}.description`)}
              />
            ))}
          </fieldset>
        )}
        {step === 'team' && (
          <fieldset className="grid gap-3 sm:grid-cols-2" disabled={busy}>
            <legend className="sr-only">{t('steps.team.title')}</legend>
            {TEAM_SIZES.map((value) => (
              <OptionCard
                key={value}
                type="radio"
                name="onboarding-team-size"
                value={value}
                checked={teamSize === value}
                onChange={() => setTeamSize(value)}
                title={t(`teamSizes.${value}`)}
              />
            ))}
          </fieldset>
        )}
        {step === 'recommendation' && (
          <RecommendationStep
            recommendation={recommendation}
            selected={selected}
            onSelect={setSelected}
            disabled={busy}
          />
        )}
        {step === 'preview' && (
          <PreviewStep
            plan={plan.data}
            isPending={plan.isPending}
            isError={plan.isError}
            onRetry={() => void plan.refetch()}
          />
        )}
      </CardContent>
      <CardFooter className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-between">
        <Button type="button" variant="ghost" onClick={() => void skip()} disabled={busy}>
          {t('actions.skip')}
        </Button>
        <div className="flex flex-col-reverse gap-2 sm:flex-row">
          {stepIndex > 0 && (
            <Button type="button" variant="outline" onClick={onBack} disabled={busy}>
              {t('actions.back')}
            </Button>
          )}
          {step === 'preview' ? (
            installId ? (
              <Button type="button" onClick={() => void complete(previewTemplate, installId)} disabled={busy}>
                {t('actions.finish')}
              </Button>
            ) : (
              <Button
                type="button"
                onClick={() => previewTemplate && void apply(previewTemplate)}
                disabled={busy || !plan.data?.valid || plan.isFetching}
              >
                {busy ? t('actions.applying') : t('actions.apply')}
              </Button>
            )
          ) : (
            <Button type="button" onClick={onNext} disabled={busy || !canContinue}>
              {nextLabel(step, chosen, t)}
            </Button>
          )}
        </div>
      </CardFooter>
    </Card>
  )
}

function nextLabel(step: Step, chosen: string | null, t: (key: 'actions.next' | 'actions.preview' | 'actions.finishScratch') => string) {
  if (step === 'recommendation') return chosen === SCRATCH ? t('actions.finishScratch') : t('actions.preview')
  return t('actions.next')
}
