import { useId, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { api, isApiError, type components } from '@/api'
import { errorMessage, notifySuccess, PageLoader, UnsavedChangesGuard } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { ErrorState } from '@/components/ui/error-state'
import { FieldError } from '@/components/ui/field-error'
import { Label } from '@/components/ui/label'
import { NativeSelect } from '@/components/ui/native-select'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { fieldViolations } from '@/features/auth/lib/api-errors'
import { usePreferences, useUpdatePreferences, type UserPreferences } from '@/features/command-palette/preferences/api'
import { selectExperience, sameSource, sourceIdentity, type ExperienceProfile, type OrganizationExperiences } from '@/features/experiences/resolution'
import { organizationExperiencesKey } from '@/features/experiences/use-experience-resolution'

type Source = components['schemas']['ExperienceSection']

export function PersonalExperiencePage() {
  const { t } = useTranslation('personal-experience')
  const preferences = usePreferences()
  const catalog = useQuery({ queryKey: organizationExperiencesKey, queryFn: () => api.get<components['schemas']['OrganizationExperiencesView']>('/v1/organization/experiences'), staleTime: 5 * 60_000 })
  const refresh = async () => { await Promise.all([preferences.refetch({ throwOnError: true }), catalog.refetch({ throwOnError: true })]) }
  return <Page>
    <PageHeader title={t('title')} description={t('description')} />
    {(preferences.isError || catalog.isError) && <ErrorState description={errorMessage(preferences.error ?? catalog.error)} onRetry={() => void refresh().catch(() => undefined)} />}
    {(preferences.fetchStatus === 'paused' || catalog.fetchStatus === 'paused') && <ErrorState description={t('offline')} onRetry={() => void refresh().catch(() => undefined)} />}
    {preferences.data && catalog.data ? <PersonalControls preferences={preferences.data} config={catalog.data.configuration ?? null} blocked={preferences.isError || catalog.isError || preferences.fetchStatus === 'paused' || catalog.fetchStatus === 'paused'} /> : !preferences.isError && !catalog.isError && preferences.fetchStatus !== 'paused' && catalog.fetchStatus !== 'paused' && <PageLoader />}
  </Page>
}

function PersonalControls({ preferences, config, blocked }: { preferences: UserPreferences; config: OrganizationExperiences | null; blocked: boolean }) {
  const { t } = useTranslation('personal-experience')
  const id = useId()
  const queryClient = useQueryClient()
  const update = useUpdatePreferences()
  const [error, setError] = useState<unknown>(null)
  const raw = preferences.stored_experience
  const { profile, diagnostics } = selectExperience(config, raw)
  const fail = (error: unknown) => {
    setError(error)
    void queryClient.invalidateQueries({ queryKey: organizationExperiencesKey })
    void queryClient.invalidateQueries({ queryKey: ['preferences'] })
  }
  const succeeded = () => {
    void queryClient.invalidateQueries({ queryKey: organizationExperiencesKey })
    notifySuccess(t('saved'))
  }
  const changeProfile = (key: string | null) => {
    setError(null)
    update.mutate({ experience: { profile: key } }, { onSuccess: succeeded, onError: fail })
  }
  return <fieldset disabled={blocked} className="grid min-w-0 gap-4">
    <legend className="sr-only">{t('title')}</legend>
    <p role="note">{t('personalOnly')}</p>
    <p role="status">{t('effective', { label: profile?.label ?? t('baseline') })}</p>
    {diagnostics.length + preferences.experience_diagnostics.length > 0 && <div role="status" className="grid gap-1 rounded-md border p-3">
      <p>{t('retained')}</p>
      {preferences.experience_diagnostics.map((item, index) => <p key={index} className="text-sm">{item.path}: {t('ignored')}</p>)}
    </div>}
    {error !== null && <FieldError>{errorMessage(error)}</FieldError>}
    <Label htmlFor={id}>{t('selection')}</Label>
    <NativeSelect id={id} value={raw.profile ?? ''} disabled={!config?.policy.customization.profile_selection || update.isPending} onChange={(event) => changeProfile(event.target.value || null)}>
      <option value="">{t('followDefault')}</option>
      {raw.profile && !config?.profiles.some((entry) => entry.profile.key === raw.profile) && <option value={raw.profile} disabled>{raw.profile} · {t('unavailable')}</option>}
      {config?.profiles.map(({ profile: item, enabled }) => {
        const allowed = enabled && config.policy.allowed_profiles.includes(item.key)
        return <option key={item.key} value={item.key} disabled={!allowed}>{item.label}{allowed ? '' : ` · ${t('unavailable')}`}</option>
      })}
    </NativeSelect>
    {!config?.policy.customization.profile_selection && <p className="text-sm text-muted-foreground">{t('selectionBlocked')}</p>}
    <Button variant="outline" disabled={update.isPending || !raw.profile} onClick={() => changeProfile(null)}>{t('resetSelection')}</Button>
    {profile ? <SectionPreferences profile={profile} config={config!} preferences={preferences} fail={fail} update={update} succeeded={succeeded} /> : <p>{t('noProfile')}</p>}
    {!profile && raw.record_detail && <Button variant="outline" disabled={update.isPending} onClick={() => update.mutate({ experience: { record_detail: null } }, { onSuccess: succeeded, onError: fail })}>{t('resetSections')}</Button>}
    <p className="text-sm text-muted-foreground">{t('otherOwners')}</p>
    <Link className="underline" to="/account/navigation">{t('navigation')}</Link>
  </fieldset>
}

function SectionPreferences({ profile, config, preferences, fail, update, succeeded }: { profile: ExperienceProfile; config: OrganizationExperiences; preferences: UserPreferences; fail: (error: unknown) => void; update: ReturnType<typeof useUpdatePreferences>; succeeded: () => void }) {
  const { t } = useTranslation('personal-experience')
  const [error, setError] = useState<unknown>(null)
  const saved = preferences.experience.record_detail
  const main = profile.record_detail.regions?.main ?? []
  const policy = config.policy.customization
  const omittable = preferences.experience_omittable_sections ?? []
  const initial = saved?.record_type === profile.record_detail.record_type ? { order: saved.order, hidden: saved.hidden } : { order: [] as Source[], hidden: [] as Source[] }
  const signature = JSON.stringify([profile, policy, initial, preferences.stored_experience.record_detail, omittable])
  const [draft, setDraft] = useState<{ signature: string; order: Source[]; hidden: Source[] } | null>(null)
  const form = draft ?? { signature, ...initial }
  const stale = form.signature !== signature
  const dirty = draft !== null
  const editable = main.length > 0
  const ordered = [...form.order.filter((source) => main.some((item) => sameSource(item, source))), ...main].filter((source, index, all) => all.findIndex((other) => sameSource(other, source)) === index)
  const save = (reset: boolean) => {
    setError(null)
    update.mutate({ experience: { record_detail: reset ? null : { record_type: profile.record_detail.record_type, order: form.order, hidden: form.hidden } } }, {
      onSuccess: () => { setDraft(null); succeeded() },
      onError: (error) => { setError(error); fail(error) },
    })
  }
  return <section className="grid gap-3 rounded-md border p-4">
    <UnsavedChangesGuard when={dirty} />
    <h2 className="font-semibold">{t('sections', { type: profile.record_detail.record_type })}</h2>
    <p className="text-sm text-muted-foreground">{t('contextHint')}</p>
    {stale && <p role="alert" className="text-warning">{t('stale')}</p>}
    {error !== null && <div role="alert"><FieldError>{errorMessage(error)}</FieldError><ul>{(isApiError(error) ? fieldViolations(error.details) : []).map((item, index) => <li key={index}>{item.field}: {item.message}</li>)}</ul></div>}
    {!editable && <p>{t('noComposition')}</p>}
    {!policy.section_order && <p>{t('orderBlocked')}</p>}
    <fieldset disabled={update.isPending || stale} className="grid min-w-0 gap-2">
      <legend className="sr-only">{t('controls')}</legend>
      <ol className="grid gap-3">
        {ordered.map((source, index) => {
          const identity = sourceIdentity(source)
          const canHide = policy.section_hidden.some((item) => sameSource(item, source)) && omittable.some((item) => sameSource(item, source))
          return <li key={identity} className="flex flex-wrap items-center gap-2">
            <span className="min-w-0 break-all">{identity}</span>
            <Button size="sm" variant="outline" disabled={!policy.section_order || index === 0} aria-label={t('upLabel', { source: identity })} onClick={() => {
              const order = [...ordered]; [order[index - 1], order[index]] = [order[index], order[index - 1]]; setDraft({ ...form, order })
            }}>{t('up')}</Button>
            <Button size="sm" variant="outline" disabled={!policy.section_order || index === ordered.length - 1} aria-label={t('downLabel', { source: identity })} onClick={() => {
              const order = [...ordered]; [order[index], order[index + 1]] = [order[index + 1], order[index]]; setDraft({ ...form, order })
            }}>{t('down')}</Button>
            <Label className="flex items-center gap-2"><Checkbox disabled={!canHide} checked={form.hidden.some((item) => sameSource(item, source))} onCheckedChange={(value) => setDraft({ ...form, hidden: value === true ? [...form.hidden, source] : form.hidden.filter((item) => !sameSource(item, source)) })} />{t('hide', { source: identity })}</Label>
            {!canHide && <span className="text-sm text-muted-foreground">{t(policy.section_hidden.some((item) => sameSource(item, source)) ? 'ownerBlocked' : 'hideBlocked')}</span>}
          </li>
        })}
      </ol>
      <Button disabled={!dirty || !editable} onClick={() => save(false)}>{t('saveSections')}</Button>
    </fieldset>
    {dirty && <Button variant="outline" disabled={update.isPending} onClick={() => { setDraft(null); setError(null) }}>{t('discard')}</Button>}
    <Button variant="outline" disabled={update.isPending || !preferences.stored_experience.record_detail} onClick={() => save(true)}>{t('resetSections')}</Button>
    <p className="text-sm text-muted-foreground">{t('resetHint')}</p>
  </section>
}
