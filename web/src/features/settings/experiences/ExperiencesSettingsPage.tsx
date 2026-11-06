import { useId, useRef, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { api, isApiError } from '@/api'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, errorMessage, notifySuccess, PageLoader, UnsavedChangesGuard } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from '@/components/ui/dialog'
import { ErrorState } from '@/components/ui/error-state'
import { FieldError } from '@/components/ui/field-error'
import { Label } from '@/components/ui/label'
import { NativeSelect } from '@/components/ui/native-select'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { fieldViolations } from '@/features/auth/lib/api-errors'
import { organizationExperiencesKey } from '@/features/experiences/use-experience-resolution'
import { RequirePermission } from '../components/RequirePermission'
import { SETTINGS_MANAGE, SYSTEM_MANAGE } from '../lib/permissions'
import { CatalogReview } from './CatalogReview'
import { ProfileEditor } from './ProfileEditor'
import { catalogPath, emptyCatalog, pruneReferences, sourceKey, type Catalog, type CatalogView, type Profile } from './model'

export function ExperiencesSettingsPage() {
  return <RequirePermission permission={SETTINGS_MANAGE}><ExperiencesSettingsContent /></RequirePermission>
}

function ExperiencesSettingsContent() {
  const { t } = useTranslation('experiences-settings')
  const query = useQuery({ queryKey: organizationExperiencesKey, queryFn: () => api.get<CatalogView>(catalogPath) })
  return <Page>
    <PageHeader title={t('title')} description={t('description')} />
    {query.data && query.isError && <FieldError>{errorMessage(query.error)}</FieldError>}
    {!query.data ? query.fetchStatus === 'paused' ? <ErrorState description={t('offline')} onRetry={() => void query.refetch()} /> : query.isError ? <ErrorState description={errorMessage(query.error)} onRetry={() => void query.refetch()} /> : <PageLoader /> : <CatalogEditor latest={query.data} refresh={() => query.refetch({ throwOnError: true })} />}
  </Page>
}

function CatalogEditor({ latest, refresh }: { latest: CatalogView; refresh: () => Promise<unknown> }) {
  const { t } = useTranslation('experiences-settings')
  const id = useId()
  const { hasPermission } = useSession()
  const queryClient = useQueryClient()
  const fileInput = useRef<HTMLInputElement>(null)
  const [snapshot, setSnapshot] = useState(latest)
  const [draft, setDraft] = useState<Catalog>(() => latest.configuration ?? emptyCatalog())
  const [editing, setEditing] = useState<{ profile?: Profile } | null>(null)
  const [review, setReview] = useState<CatalogView | null>(null)
  const [error, setError] = useState<unknown>(null)
  const [reload, setReload] = useState(false)
  const before = snapshot.configuration ?? emptyCatalog()
  const dirty = JSON.stringify(draft) !== JSON.stringify(before)
  const stale = snapshot.updated_at !== latest.updated_at || review && review.updated_at !== snapshot.updated_at
  const update = (next: Catalog) => { setDraft(next); setReview(null); setError(null) }
  const validate = useMutation({
    mutationFn: (body: unknown) => api.post<CatalogView>(`${catalogPath}/validate`, body),
    onSuccess: (report) => { setReview(report); setError(null) },
    onError: setError,
  })
  const importing = useMutation({
    mutationFn: async (file: File) => {
      if (file.size > 512 * 1024) throw new Error(t('fileTooLarge'))
      let body: unknown
      try { body = JSON.parse(await file.text()) } catch { throw new Error(t('invalidJson')) }
      return api.post<CatalogView>(`${catalogPath}/validate`, body)
    },
    onSuccess: (report) => { if (report.configuration) { update(report.configuration); setReview(report) } },
    onError: setError,
  })
  const save = useMutation({
    mutationFn: (body: Catalog) => api.put<CatalogView>(catalogPath, body, { headers: { 'If-Match': snapshot.updated_at } }),
    onSuccess: (saved) => {
      setSnapshot(saved); setDraft(saved.configuration ?? emptyCatalog()); setReview(null); setError(null)
      queryClient.setQueryData(organizationExperiencesKey, saved)
      void queryClient.invalidateQueries({ queryKey: ['preferences'] })
      notifySuccess(t('saved'))
    },
    onError: setError,
  })
  const busy = validate.isPending || importing.isPending || save.isPending
  const errors = isApiError(error) ? fieldViolations(error.details) : []
  const sources = [...new Map(draft.profiles.filter((entry) => entry.enabled && draft.policy.allowed_profiles.includes(entry.profile.key)).flatMap((entry) => entry.profile.record_detail.regions?.main ?? []).map((source) => [sourceKey(source), source])).values()]
  const customization = draft.policy.customization
  const togglePolicy = (key: 'profile_selection' | 'navigation' | 'section_order', value: boolean) => update({ ...draft, policy: { ...draft.policy, customization: { ...customization, [key]: value } } })
  return <div className="grid gap-4">
    <UnsavedChangesGuard when={dirty || editing !== null} />
    <p role="note" className="rounded-md border p-3">{t('sharedWarning')}</p>
    {stale && <p role="alert" className="text-warning">{t('stale')}</p>}
    {error !== null && <div role="alert" className="grid gap-1">
      <FieldError>{isApiError(error) && error.status === 409 ? t('conflict') : errorMessage(error)}</FieldError>
      <ul>{errors.map((item, index) => <li key={index} className="break-words text-sm text-destructive">{item.field}: {item.message}</li>)}</ul>
    </div>}
    <fieldset disabled={busy} className="grid min-w-0 gap-4">
      <legend className="sr-only">{t('title')}</legend>
      <div className="flex flex-wrap gap-2">
        <Button type="button" onClick={() => setEditing({})}>{t('addProfile')}</Button>
        <Button type="button" variant="outline" onClick={() => fileInput.current?.click()}>{t('import')}</Button>
        <input ref={fileInput} type="file" accept="application/json,.json" className="sr-only" aria-label={t('import')} onChange={(event) => {
          const file = event.target.files?.[0]; event.target.value = ''; if (file) { setError(null); setReview(null); importing.mutate(file) }
        }} />
        <Button type="button" variant="outline" onClick={() => setReload(true)}>{t('reload')}</Button>
      </div>
      <p className="text-sm text-muted-foreground">{t('importHint')}</p>
      {draft.profiles.length === 0 && <p>{t('empty')}</p>}
      <ul className="grid gap-3">
        {draft.profiles.map((entry, index) => <li key={entry.profile.key} className="grid gap-3 rounded-md border p-4">
          <h2 className="break-words font-semibold">{entry.profile.label} <span className="text-sm font-normal text-muted-foreground">{entry.profile.key}</span></h2>
          <p className="break-words text-sm">{entry.profile.record_detail.record_type} · {entry.profile.frame.module} / {entry.profile.record_detail.presentation.module}</p>
          <div className="flex flex-wrap gap-4">
            <Label className="flex items-center gap-2"><Checkbox checked={entry.enabled} onCheckedChange={(value) => update(pruneReferences({ ...draft, profiles: draft.profiles.map((item, position) => position === index ? { ...item, enabled: value === true } : item) }))} />{t('enabled')}</Label>
            <Label className="flex items-center gap-2"><Checkbox checked={draft.policy.allowed_profiles.includes(entry.profile.key)} disabled={!entry.enabled} onCheckedChange={(value) => update(pruneReferences({ ...draft, policy: { ...draft.policy, allowed_profiles: value === true ? [...draft.policy.allowed_profiles, entry.profile.key] : draft.policy.allowed_profiles.filter((key) => key !== entry.profile.key) } }))} />{t('allowed')}</Label>
          </div>
          <div className="flex gap-2">
            <Button type="button" variant="outline" size="sm" onClick={() => setEditing({ profile: entry.profile })}>{t('edit')}</Button>
            <Button type="button" variant="outline" size="sm" onClick={() => update(pruneReferences({ ...draft, profiles: draft.profiles.filter((_, position) => position !== index) }))}>{t('remove')}</Button>
          </div>
        </li>)}
      </ul>
      <Label htmlFor={`${id}-default`}>{t('default')}</Label>
      <NativeSelect id={`${id}-default`} value={draft.default_profile ?? ''} onChange={(event) => update({ ...draft, default_profile: event.target.value || null })}>
        <option value="">{t('baseline')}</option>
        {draft.profiles.filter((entry) => entry.enabled && draft.policy.allowed_profiles.includes(entry.profile.key)).map(({ profile }) => <option key={profile.key} value={profile.key}>{profile.label}</option>)}
      </NativeSelect>
      <p className="text-sm text-muted-foreground">{t('defaultHint')}</p>
      <fieldset className="grid gap-3 rounded-md border p-4">
        <legend>{t('personalOptions')}</legend>
        <Label className="flex items-center gap-2"><Checkbox checked={customization.profile_selection} onCheckedChange={(value) => togglePolicy('profile_selection', value === true)} />{t('allowSelection')}</Label>
        <Label className="flex items-center gap-2"><Checkbox checked={customization.navigation} onCheckedChange={(value) => togglePolicy('navigation', value === true)} />{t('allowNavigation')}</Label>
        <Label className="flex items-center gap-2"><Checkbox checked={customization.section_order} onCheckedChange={(value) => togglePolicy('section_order', value === true)} />{t('allowOrder')}</Label>
        <p className="text-sm text-muted-foreground">{t('hideHint')}</p>
        {sources.map((source) => <Label key={sourceKey(source)} className="flex items-center gap-2"><Checkbox checked={customization.section_hidden.some((item) => sourceKey(item) === sourceKey(source))} onCheckedChange={(value) => update({ ...draft, policy: { ...draft.policy, customization: { ...customization, section_hidden: value === true ? [...customization.section_hidden, source] : customization.section_hidden.filter((item) => sourceKey(item) !== sourceKey(source)) } } })} />{sourceKey(source)}</Label>)}
      </fieldset>
      <Button type="button" disabled={!dirty || Boolean(stale)} onClick={() => { setError(null); validate.mutate(draft) }}>{t('validate')}</Button>
    </fieldset>
    {busy && <p role="status">{t('working')}</p>}
    {review && <>
      {!busy && error === null && <p role="status">{t('validated')}</p>}
      <CatalogReview before={before} report={review} />
      <Button type="button" disabled={busy || !dirty || Boolean(stale) || error !== null} onClick={() => { if (review.configuration) save.mutate(review.configuration) }}>{t('apply')}</Button>
    </>}
    <p className="text-sm text-muted-foreground">{t('ownerHint')}</p>
    <Link className="text-sm underline" to="/settings">{t('ownerTools')}</Link>
    {hasPermission(SYSTEM_MANAGE) && <Link className="text-sm underline" to="/integrations/modules">{t('manageCode')}</Link>}
    <Dialog open={editing !== null} onOpenChange={(open) => { if (!open) setEditing(null) }}>
      {editing && <DialogContent size="lg">
        <DialogHeader><DialogTitle>{t(editing.profile ? 'edit' : 'addProfile')}</DialogTitle><DialogDescription>{t('draftOnly')}</DialogDescription></DialogHeader>
        <ProfileEditor initial={editing.profile} existing={draft.profiles.map(({ profile }) => profile.key)} onCancel={() => setEditing(null)} onDone={(profile) => {
          update(pruneReferences({ ...draft, profiles: editing.profile ? draft.profiles.map((entry) => entry.profile.key === profile.key ? { ...entry, profile } : entry) : [...draft.profiles, { enabled: true, profile }], policy: editing.profile ? draft.policy : { ...draft.policy, allowed_profiles: [...draft.policy.allowed_profiles, profile.key] } })); setEditing(null)
        }} />
      </DialogContent>}
    </Dialog>
    <ConfirmDialog open={reload} onOpenChange={setReload} title={t('reload')} description={t('reloadHint')} onConfirm={async () => {
      await refresh()
      const current = queryClient.getQueryData<CatalogView>(organizationExperiencesKey)
      if (current) { setSnapshot(current); update(current.configuration ?? emptyCatalog()) }
    }} />
  </div>
}
