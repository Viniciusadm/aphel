import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQueryClient } from '@tanstack/react-query'
import { notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { ErrorState } from '@/components/ui/error-state'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { lifecycleApi, lifecycleKeys, useLifecycleSettings } from '../api'
import { TRANSITION_TRIGGERS } from '../constants'
import { lifecycleErrorMessage } from '../errors'
import type { LifecycleSettings, LifecycleStageOption, LifecycleTransitionRule } from '../types'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'
import { Field } from '@/components/ui/field'

interface Draft {
  rules: LifecycleTransitionRule[]
  disqualifiedStage: string
}

function draftFrom(settings: LifecycleSettings): Draft {
  return {
    rules: TRANSITION_TRIGGERS.flatMap((trigger) => settings.rules.find((rule) => rule.trigger === trigger) ?? []),
    disqualifiedStage: settings.disqualified_stage ?? '',
  }
}

export function TransitionRulesSection() {
  const settings = useLifecycleSettings()
  if (settings.isPending) {
    return (
      <div className="grid gap-2" aria-busy="true">
        <Skeleton className="h-24 w-full" />
        <Skeleton className="h-24 w-full" />
      </div>
    )
  }
  if (settings.isError) return <ErrorState onRetry={() => void settings.refetch()} />
  return <RulesForm key={settings.data.updated_at ?? 'defaults'} settings={settings.data} />
}

function RulesForm({ settings }: { settings: LifecycleSettings }) {
  const { t } = useTranslation('leads')
  const queryClient = useQueryClient()
  const [draft, setDraft] = useState<Draft>(() => draftFrom(settings))
  const [saving, setSaving] = useState(false)
  const stages = settings.stages
  const activeStages = stages.filter((stage) => stage.is_active)

  const updateRule = (trigger: string, patch: Partial<LifecycleTransitionRule>) =>
    setDraft((current) => ({
      ...current,
      rules: current.rules.map((rule) => (rule.trigger === trigger ? { ...rule, ...patch } : rule)),
    }))

  const save = async () => {
    setSaving(true)
    try {
      const saved = await lifecycleApi.saveSettings(
        { rules: draft.rules, disqualified_stage: draft.disqualifiedStage || null },
        settings.updated_at,
      )
      queryClient.setQueryData(lifecycleKeys.settings(), saved)
      notifySuccess(t('settings.saved'))
    } catch (error) {
      toast({ title: lifecycleErrorMessage(error, t), variant: 'destructive' })
    } finally {
      setSaving(false)
    }
  }

  return (
    <Page>
      <PageHeader level="section" title={t('settings.rules.title')} description={t('settings.rules.description')} />
      <div className="grid gap-3">
        {draft.rules.map((rule) => (
          <RuleCard
            key={rule.trigger}
            rule={rule}
            stages={stages}
            activeStages={activeStages}
            onChange={(patch) => updateRule(rule.trigger, patch)}
          />
        ))}
      </div>
      <div className="grid gap-2.5 rounded-md border bg-card p-4">
        <Label htmlFor="lifecycle-disqualified-stage">{t('settings.disqualified.title')}</Label>
        <p className="text-xs text-muted-foreground">{t('settings.disqualified.description')}</p>
        <NativeSelect
          id="lifecycle-disqualified-stage"
          className="max-w-xs"
          value={draft.disqualifiedStage}
          onChange={(event) => setDraft((current) => ({ ...current, disqualifiedStage: event.target.value }))}
        >
          <option value="">{t('settings.disqualified.none')}</option>
          {activeStages.map((stage) => (
            <option key={stage.value} value={stage.value}>
              {stage.label}
            </option>
          ))}
        </NativeSelect>
      </div>
      <div>
        <Button onClick={() => void save()} disabled={saving}>
          {t('settings.save')}
        </Button>
      </div>
    </Page>
  )
}

function RuleCard({
  rule,
  stages,
  activeStages,
  onChange,
}: {
  rule: LifecycleTransitionRule
  stages: LifecycleStageOption[]
  activeStages: LifecycleStageOption[]
  onChange: (patch: Partial<LifecycleTransitionRule>) => void
}) {
  const { t } = useTranslation('leads')
  const prefix = `lifecycle-rule-${rule.trigger}`
  const toggleFrom = (value: string, checked: boolean) =>
    onChange({
      from_stages: checked
        ? [...rule.from_stages.filter((stage) => stage !== value), value]
        : rule.from_stages.filter((stage) => stage !== value),
    })

  return (
    <fieldset className="grid gap-3 rounded-md border bg-card p-4">
      <legend className="sr-only">{t(`settings.rules.triggers.${rule.trigger}.label`)}</legend>
      <div className="flex items-start gap-3">
        <Checkbox
          id={`${prefix}-enabled`}
          checked={rule.enabled}
          onCheckedChange={(checked) => onChange({ enabled: checked === true })}
        />
        <div className="grid gap-0.5">
          <Label htmlFor={`${prefix}-enabled`} className="font-medium">
            {t(`settings.rules.triggers.${rule.trigger}.label`)}
          </Label>
          <p className="text-xs text-muted-foreground">{t(`settings.rules.triggers.${rule.trigger}.description`)}</p>
        </div>
      </div>
      <Field label={t('settings.rules.target')} className="sm:max-w-xs">
        {(fieldControl) => <NativeSelect
          {...fieldControl}
          value={rule.target_stage}
          disabled={!rule.enabled}
          onChange={(event) => onChange({ target_stage: event.target.value })}
        >
          {activeStages.map((stage) => (
            <option key={stage.value} value={stage.value}>
              {stage.label}
            </option>
          ))}
        </NativeSelect>}
      </Field>
      <div className="grid gap-1.5">
        <span className="text-sm font-medium">{t('settings.rules.from')}</span>
        <p className="text-xs text-muted-foreground">{t('settings.rules.fromDescription')}</p>
        <div className="flex flex-wrap gap-x-4 gap-y-2">
          <label className="inline-flex items-center gap-2 text-sm">
            <Checkbox
              checked={rule.from_empty}
              disabled={!rule.enabled}
              onCheckedChange={(checked) => onChange({ from_empty: checked === true })}
            />
            {t('settings.rules.noStage')}
          </label>
          {stages
            .filter((stage) => stage.value !== rule.target_stage)
            .map((stage) => (
              <label key={stage.value} className="inline-flex items-center gap-2 text-sm">
                <Checkbox
                  checked={rule.from_stages.includes(stage.value)}
                  disabled={!rule.enabled}
                  onCheckedChange={(checked) => toggleFrom(stage.value, checked === true)}
                />
                {stage.label}
              </label>
            ))}
        </div>
      </div>
    </fieldset>
  )
}
