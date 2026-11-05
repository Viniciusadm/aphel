import { zodResolver } from '@hookform/resolvers/zod'
import { useQueryClient } from '@tanstack/react-query'
import { useCallback, useEffect, useMemo, useState } from 'react'
import { useForm, useWatch } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { NETWORK_ERROR_CODE, isApiError } from '@/api'
import { SessionProvider } from '@/app/session/SessionProvider'
import { storeSession } from '@/app/session/session-query'
import { fieldViolations } from '@/features/auth/lib/api-errors'
import {
  DEFAULT_CURRENCY,
  DEFAULT_TIMEZONE,
  browserTimezone,
  timezoneOptions,
} from '@/features/settings/organization/lib/preferences'
import type { SetupTaskContribution } from '@/kernel/modules/define'
import { switchInstallerLanguage } from '@/kernel/modules/i18n'
import { ModulesSync } from '@/kernel/modules/ModulesSync'
import { contributions, sortContributions } from '@/kernel/modules/registry'
import { SetupFrameContext, SetupStepFrame } from '@/kernel/setup'
import { INSTALLER_STATUS_KEY, install, useInstallerCatalog, type InstallerMe } from './api'
import { SetupLoadingCard } from './components/SetupLoadingCard'
import { SetupProgress } from './components/SetupProgress'
import { SetupShell } from './components/SetupShell'
import { EMPTY_BUSINESS, type BusinessAnswers } from './lib/business'
import { clearDraft, readDraft, writeDraft } from './lib/draft'
import { ORGANIZATION_FIELDS, SERVER_FIELDS, SERVER_MESSAGES, setupSchema, type SetupValues } from './lib/form'
import { EMPTY_CHOICE, dependencyClosure, preselect, withIncluded, withoutSecrets, type InstallerCatalog, type ModuleChoice } from './lib/modules'
import { DRAFT_STEPS, isDraftStep, useStepLabel } from './lib/steps'
import { organizationSummary } from './lib/summary'
import { AccountStep } from './steps/AccountStep'
import { BusinessStep } from './steps/BusinessStep'
import { OrganizationStep } from './steps/OrganizationStep'
import { EmailStep } from './steps/EmailStep'
import { FeaturesStep } from './steps/FeaturesStep'
import { ReadyStep, type ReadyResult } from './steps/ReadyStep'
import { RestoreStep } from './steps/RestoreStep'

const KEYS = [...DRAFT_STEPS, 'email', 'ready']

const allTasks = () => sortContributions(contributions<SetupTaskContribution>('setupTasks'))

export function InstallWizard() {
  const { t, i18n } = useTranslation('setup')
  const label = useStepLabel()
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const catalogQuery = useInstallerCatalog()
  const [rawCatalog, setCatalog] = useState<InstallerCatalog | null>(null)
  if (catalogQuery.data && rawCatalog === null) setCatalog(catalogQuery.data)

  const [draft] = useState(readDraft)
  const [initialTimezone] = useState(() => {
    const detected = browserTimezone()
    return timezoneOptions(DEFAULT_TIMEZONE).includes(detected) ? detected : DEFAULT_TIMEZONE
  })
  const form = useForm<SetupValues>({
    resolver: zodResolver(setupSchema),
    defaultValues: {
      organizationName: '',
      timezone: initialTimezone,
      currency: DEFAULT_CURRENCY,
      locale: i18n.language,
      adminName: '',
      adminEmail: '',
      ...draft?.values,
      adminPassword: '',
    },
  })
  const [step, setStep] = useState<string>(isDraftStep(draft?.step) ? draft.step : 'organization')
  const [business, setBusiness] = useState<BusinessAnswers>(draft?.business ?? EMPTY_BUSINESS)
  const catalog = useMemo(
    () => (rawCatalog ? withIncluded(rawCatalog, business.business_type) : null),
    [rawCatalog, business.business_type],
  )
  const [choice, setChoice] = useState<ModuleChoice>(draft?.choice ?? EMPTY_CHOICE)
  const [preselectedFor, setPreselectedFor] = useState<string | null>(draft?.preselectedFor ?? null)
  const [error, setError] = useState<string | null>(null)
  const [restoring, setRestoring] = useState(false)
  const [submitting, setSubmitting] = useState(false)
  const [installed, setInstalled] = useState<InstallerMe | null>(null)
  const [progress, setProgress] = useState<string | null>(null)
  const [results, setResults] = useState<ReadyResult[]>([])
  const [tasks] = useState(allTasks)

  useEffect(() => {
    const locale = draft?.values?.locale
    if (locale && locale !== i18n.language) void switchInstallerLanguage(locale)
  }, [draft, i18n])

  const persist = useCallback(() => {
    if (installed) return
    const { adminPassword: _password, ...values } = form.getValues()
    writeDraft({ step, values, business, choice: withoutSecrets(catalog ?? undefined, choice), preselectedFor })
  }, [business, catalog, choice, form, installed, preselectedFor, step])

  const watched = useWatch({ control: form.control })
  useEffect(() => persist(), [persist, watched])

  const enabledBeforeInstall = useMemo(
    () => (catalog ? dependencyClosure(catalog.modules, choice.modules) : new Set<string>()),
    [catalog, choice.modules],
  )
  const hasProfiles = Boolean(catalog?.profiles.length)
  const hasModules = Boolean(catalog?.modules.some((module) => !module.required))
  const keys = KEYS.filter((key) => (key !== 'business' || hasProfiles) && (key !== 'features' || hasModules))
  const steps = keys.map((key) => ({ key, label: t(`steps.${key as 'organization'}`) }))
  const index = Math.max(0, keys.indexOf(step))

  const goTo = (next: string) => {
    setError(null)
    setStep(next)
    window.scrollTo({ top: 0 })
  }
  const advance = () => goTo(keys[index + 1] ?? 'ready')

  const businessLabel = business.business_type ? t(`business.types.${business.business_type}.label`) : ''

  const nextFromOrganization = async () => {
    if (await form.trigger([...ORGANIZATION_FIELDS])) goTo(hasProfiles ? 'business' : hasModules ? 'features' : 'account')
  }

  const startBlank = () => {
    setError(null)
    setBusiness(EMPTY_BUSINESS)
    setChoice(EMPTY_CHOICE)
    setPreselectedFor(null)
    goTo('features')
  }

  const nextFromBusiness = () => {
    if (!business.business_type) {
      setError(t('business.required'))
      return
    }
    if (catalog && preselectedFor !== business.business_type) {
      setChoice(preselect(withIncluded(catalog, business.business_type), business.business_type))
      setPreselectedFor(business.business_type)
    }
    goTo('features')
  }

  const failInstall = (failure: unknown) => {
    if (!isApiError(failure)) {
      setError(t('errors.generic'))
      return
    }
    if (failure.status === 404) {
      queryClient.setQueryData(INSTALLER_STATUS_KEY, false)
      void navigate('/login', { replace: true })
      return
    }
    if (failure.code === 'email_taken') {
      form.setError('adminEmail', { message: 'errors.emailTaken' })
      return
    }
    const targets: { step: string; message: string | null }[] = []
    for (const violation of fieldViolations(failure.details)) {
      const field = SERVER_FIELDS[violation.field]
      if (field) {
        form.setError(field, { message: SERVER_MESSAGES[field] })
        targets.push({ step: (ORGANIZATION_FIELDS as readonly string[]).includes(field) ? 'organization' : 'account', message: null })
      } else if (violation.field.startsWith('business')) {
        targets.push({ step: 'business', message: t('errors.business') })
      } else if (violation.field === 'modules' || violation.field.startsWith('module_settings')) {
        targets.push({ step: 'features', message: t('errors.features') })
      }
    }
    const target = targets.sort((left, right) => keys.indexOf(left.step) - keys.indexOf(right.step))[0]
    if (target) {
      setStep(target.step)
      setError(target.message)
      return
    }
    setError(failure.code === NETWORK_ERROR_CODE ? t('errors.network') : t('errors.generic'))
  }

  const createWorkspace = async () => {
    if (!catalog) return
    if (!(await form.trigger())) {
      if (ORGANIZATION_FIELDS.some((field) => form.getFieldState(field).invalid)) goTo('organization')
      return
    }
    const values = form.getValues()
    setError(null)
    setSubmitting(true)
    try {
      const me = await install({
        ...choice,
        business: hasProfiles ? business : undefined,
        organization_name: values.organizationName.trim(),
        timezone: values.timezone,
        currency: values.currency,
        locale: values.locale,
        admin_name: values.adminName.trim(),
        admin_email: values.adminEmail.trim(),
        admin_password: values.adminPassword,
        public_base_url: window.location.origin,
      })
      clearDraft()
      storeSession(queryClient, me)
      queryClient.setQueryData(INSTALLER_STATUS_KEY, false)
      form.resetField('adminPassword')
      const enabled = dependencyClosure(catalog.modules, choice.modules)
      const outcomes: ReadyResult[] = [{ ok: true, text: t('ready.created', { name: values.organizationName.trim() }) }]
      for (const task of tasks.filter((item) => item.module === 'kernel' || enabled.has(item.module) || catalog.modules.some((module) => module.key === item.module && module.required))) {
        setProgress(label(task.value.progressLabel))
        try {
          const result = await task.value.run()
          if (result) outcomes.push({ ok: true, text: result.summary })
        } catch {
          outcomes.push({ ok: false, text: label(task.value.failureLabel) })
        }
      }
      setResults(outcomes)
      setInstalled(me)
      setProgress(null)
      advance()
    } catch (failure) {
      failInstall(failure)
    } finally {
      setSubmitting(false)
    }
  }

  const summary = () => {
    const values = form.getValues()
    const optional = catalog?.modules.filter((item) => !item.required) ?? []
    const count = optional.filter((item) => enabledBeforeInstall.has(item.key)).length
    return [
      {
        key: 'organization',
        label: t('account.summaryOrganization'),
        value: organizationSummary(values, i18n.language, t(`locales.${values.locale as 'en'}`)),
        onEdit: () => goTo('organization'),
      },
      {
        key: 'business',
        label: t('account.summaryBusiness'),
        value: [
          businessLabel,
          business.offering.map((item) => t(`business.offerings.${item}.label`)).join(', '),
          business.team_size ? t(`business.teamSizes.${business.team_size}`) : '',
        ]
          .filter(Boolean)
          .join(' · '),
        onEdit: () => goTo('business'),
      },
      { key: 'features', label: t('account.summaryFeatures'), value: t('features.count', { count }), onEdit: () => goTo('features') },
    ].filter((item) => (item.key !== 'business' || hasProfiles) && (item.key !== 'features' || hasModules))
  }

  const catalogGate = (title: string) =>
    catalogQuery.isError && !catalog ? (
      <SetupStepFrame
        title={title}
        error={t('errors.statusFailed')}
        primary={{ label: t('retry'), onClick: () => void catalogQuery.refetch() }}
      />
    ) : (
      <SetupLoadingCard label={t('loading')} />
    )

  const renderStep = () => {
    if (restoring) return <RestoreStep />
    if (step === 'organization') {
      return <OrganizationStep form={form} error={error} onNext={() => void nextFromOrganization()} onRestore={() => setRestoring(true)} />
    }
    if (step === 'business') return <BusinessStep value={business} onChange={setBusiness} error={error} onNext={nextFromBusiness} onBlank={startBlank} />
    if (step === 'features') {
      if (!catalog) return catalogGate(t('features.title'))
      return (
        <FeaturesStep
          catalog={catalog}
          businessType={business.business_type}
          businessLabel={businessLabel}
          value={choice}
          onChange={setChoice}
          error={error}
          onNext={() => goTo('account')}
        />
      )
    }
    if (step === 'account') {
      if (!catalog) return catalogGate(t('account.title'))
      return (
        <AccountStep
          form={form}
          summary={summary()}
          error={error}
          submitting={submitting}
          progress={progress}
          onSubmit={() => void createWorkspace()}
        />
      )
    }
    if (step === 'email') return <EmailStep onDone={advance} />
    return <ReadyStep results={results} onFinish={() => void navigate('/', { replace: true })} />
  }

  const onBack = restoring
    ? () => setRestoring(false)
    : !installed && index > 0 && isDraftStep(step) && !submitting
      ? () => goTo(keys[index - 1])
      : undefined

  const body = <div key={restoring ? 'restore' : step}>{renderStep()}</div>

  return (
    <SetupShell progress={restoring ? undefined : <SetupProgress steps={steps} current={index} />}>
      <SetupFrameContext.Provider value={{ onBack }}>
        {installed ? (
          <SessionProvider me={installed}>
            <ModulesSync />
            {body}
          </SessionProvider>
        ) : (
          body
        )}
      </SetupFrameContext.Provider>
    </SetupShell>
  )
}
