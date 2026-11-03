import { useQueryClient } from '@tanstack/react-query'
import { Loader2 } from 'lucide-react'
import { useId, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { UnsavedChangesGuard, notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { emailApi, emailKeys, useCaptureSettings, useTrackingSettings } from '../api'
import { MAX_HISTORY_DAYS, validHistoryDays } from '../lib/mailbox-form'
import type { CaptureSettings, TrackingSettings } from '../types'
import { Field } from '@/components/ui/field'

export function PreferencesPanel() {
  return (
    <div className="grid gap-6">
      <TrackingSection />
      <CaptureSection />
    </div>
  )
}

function Section({ title, description, children }: { title: string; description: string; children: ReactNode }) {
  return (
    <section className="grid gap-4 rounded-lg border p-4">
      <div className="grid gap-1">
        <h3 className="text-base font-semibold">{title}</h3>
        <p className="text-sm text-muted-foreground">{description}</p>
      </div>
      {children}
    </section>
  )
}

function ToggleRow({
  id,
  label,
  description,
  checked,
  disabled,
  onChange,
}: {
  id: string
  label: string
  description: string
  checked: boolean
  disabled?: boolean
  onChange: (checked: boolean) => void
}) {
  return (
    <div className="flex items-start gap-3">
      <Checkbox
        id={id}
        checked={checked}
        disabled={disabled}
        className="mt-0.5"
        onCheckedChange={(value) => onChange(value === true)}
      />
      <Field id={id} label={label}>
        <p className="text-sm text-muted-foreground">{description}</p>
      </Field>
    </div>
  )
}

function SectionSkeleton() {
  return (
    <div className="grid gap-2" aria-busy="true">
      <Skeleton className="h-6 w-1/2" />
      <Skeleton className="h-6 w-2/3" />
      <Skeleton className="h-6 w-1/3" />
    </div>
  )
}

function TrackingSection() {
  const { t } = useTranslation('email')
  const query = useTrackingSettings()
  return (
    <Section title={t('tracking.title')} description={t('tracking.description')}>
      {query.isPending && <SectionSkeleton />}
      {query.isError && <ErrorState onRetry={() => void query.refetch()} />}
      {query.isSuccess && <TrackingForm key={JSON.stringify(query.data)} initial={query.data} />}
    </Section>
  )
}

function TrackingForm({ initial }: { initial: TrackingSettings }) {
  const { t } = useTranslation('email')
  const formId = useId()
  const queryClient = useQueryClient()
  const [values, setValues] = useState(initial)
  const [saving, setSaving] = useState(false)
  const dirty = JSON.stringify(values) !== JSON.stringify(initial)
  const anyEnabled = values.opens_enabled || values.clicks_enabled

  const save = async () => {
    setSaving(true)
    try {
      const saved = await emailApi.saveTracking({ ...values, default_on: anyEnabled && values.default_on })
      queryClient.setQueryData(emailKeys.tracking(), saved)
      notifySuccess(t('tracking.saved'))
    } catch (error) {
      notifyError(error)
    } finally {
      setSaving(false)
    }
  }

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        void save()
      }}
    >
      <UnsavedChangesGuard when={dirty} />
      <ToggleRow
        id={`${formId}-opens`}
        label={t('tracking.opens')}
        description={t('tracking.opensDescription')}
        checked={values.opens_enabled}
        onChange={(checked) => setValues((current) => ({ ...current, opens_enabled: checked }))}
      />
      <ToggleRow
        id={`${formId}-clicks`}
        label={t('tracking.clicks')}
        description={t('tracking.clicksDescription')}
        checked={values.clicks_enabled}
        onChange={(checked) => setValues((current) => ({ ...current, clicks_enabled: checked }))}
      />
      <ToggleRow
        id={`${formId}-default`}
        label={t('tracking.defaultOn')}
        description={t('tracking.defaultOnDescription')}
        checked={anyEnabled && values.default_on}
        disabled={!anyEnabled}
        onChange={(checked) => setValues((current) => ({ ...current, default_on: checked }))}
      />
      <p className="text-xs text-muted-foreground">{t('tracking.privacyNote')}</p>
      <div className="flex justify-end">
        <Button type="submit" disabled={!dirty || saving} className="w-full sm:w-auto">
          {saving && <Loader2 className="animate-spin" aria-hidden="true" />}
          {t('actions.save')}
        </Button>
      </div>
    </form>
  )
}

function CaptureSection() {
  const { t } = useTranslation('email')
  const query = useCaptureSettings()
  return (
    <Section title={t('capture.title')} description={t('capture.description')}>
      {query.isPending && <SectionSkeleton />}
      {query.isError && <ErrorState onRetry={() => void query.refetch()} />}
      {query.isSuccess && <CaptureForm key={JSON.stringify(query.data)} initial={query.data} />}
    </Section>
  )
}

function CaptureForm({ initial }: { initial: CaptureSettings }) {
  const { t } = useTranslation('email')
  const formId = useId()
  const queryClient = useQueryClient()
  const [values, setValues] = useState(initial)
  const [historyDays, setHistoryDays] = useState(String(initial.initial_history_days))
  const [saving, setSaving] = useState(false)
  const historyValid = validHistoryDays(historyDays)
  const dirty =
    JSON.stringify(values) !== JSON.stringify(initial) || historyDays !== String(initial.initial_history_days)

  const save = async () => {
    if (!historyValid) return
    setSaving(true)
    try {
      const saved = await emailApi.saveCapture({ ...values, initial_history_days: Number(historyDays) })
      queryClient.setQueryData(emailKeys.capture(), saved)
      notifySuccess(t('capture.saved'))
    } catch (error) {
      notifyError(error)
    } finally {
      setSaving(false)
    }
  }

  return (
    <form
      className="grid gap-4"
      noValidate
      onSubmit={(event) => {
        event.preventDefault()
        void save()
      }}
    >
      <UnsavedChangesGuard when={dirty} />
      <ToggleRow
        id={`${formId}-people`}
        label={t('capture.createContacts')}
        description={t('capture.createContactsDescription')}
        checked={values.create_contacts}
        onChange={(checked) => setValues((current) => ({ ...current, create_contacts: checked }))}
      />
      <ToggleRow
        id={`${formId}-organizations`}
        label={t('capture.createCompanies')}
        description={t('capture.createCompaniesDescription')}
        checked={values.create_companies}
        onChange={(checked) => setValues((current) => ({ ...current, create_companies: checked }))}
      />
      <ToggleRow
        id={`${formId}-deals`}
        label={t('capture.linkOpenDeals')}
        description={t('capture.linkOpenDealsDescription')}
        checked={values.link_open_deals}
        onChange={(checked) => setValues((current) => ({ ...current, link_open_deals: checked }))}
      />
      <Field label={t('capture.initialHistoryDays')} className="max-w-xs">
        {(control) => <>
          <Input placeholder={t('placeholders.historyDays')}
          {...control}
          type="number"
          inputMode="numeric"
          min={1}
          max={MAX_HISTORY_DAYS}
          value={historyDays}
          aria-invalid={!historyValid || undefined}
          aria-describedby={`${formId}-history-hint`}
          onChange={(event) => setHistoryDays(event.target.value)}
        />
        <p id={`${formId}-history-hint`} className={historyValid ? 'text-xs text-muted-foreground' : 'text-xs text-destructive'}>
          {historyValid
            ? t('capture.initialHistoryHint')
            : t('mailboxes.form.historyInvalid', { max: MAX_HISTORY_DAYS })}
        </p>
        </>}
      </Field>
      <div className="flex justify-end">
        <Button type="submit" disabled={!dirty || saving || !historyValid} className="w-full sm:w-auto">
          {saving && <Loader2 className="animate-spin" aria-hidden="true" />}
          {t('actions.save')}
        </Button>
      </div>
    </form>
  )
}
