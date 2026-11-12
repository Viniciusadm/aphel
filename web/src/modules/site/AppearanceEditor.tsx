import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { api } from '@/api'
import { notifyError, notifySuccess, UnsavedChangesGuard, UnsavedChangesScope, UnsavedChangesDialog } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { NativeSelect } from '@/components/ui/native-select'
import { ToggleField } from '@/components/ui/toggle-field'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { BlockImageField } from '@/sdk'
import { AppearanceFrameCollect, AppearanceFrameDraft, type FrameCollector, type SiteModuleSettings } from './siteSettings'
import { HeaderCard, FooterCard } from './SiteFrameCards'
import { ThemePresets } from './ThemePresets'
import { StylesEditor, type FontStack } from './StylesEditor'
import { mergeStyles } from './stylesDocument'

type Values = Record<string, unknown>
interface Property { key: string; group: string; kind: string; default: string | boolean | null; options: string[] }
interface Target { entity: string; target: string; label: string | null; path: string }
interface Catalog { frame: { configuration: { site: SiteModuleSettings }; overview: { base_url: string; default_language: string } };  theme_properties: string[]; theme_defaults: Values; themes: { key: string; name: string }[]; theme_catalogs: Record<string, { properties: string[]; defaults: Values }>; available: boolean; properties: Property[]; presets: { key: string; values: Values }[]; variations: Record<string, Values>; theme_variations: { key: string; name: string }[]; theme_styles: Values | null; base_styles: Values; font_stacks: FontStack[]; targets: Target[] }
interface AppearanceRecord { id: string; draft_values?: string | null; values?: string | null; updated_at: string; draft_logo?: string | null; draft_dark_logo?: string | null; draft_icon?: string | null; logo?: string | null; dark_logo?: string | null; icon?: string | null }
const recordUrl = (entity: string) => `/v1/entities/${entity}/records`
const parsed = (text?: string | null): Values => text ? JSON.parse(text) as Values : {}

export type AppearanceEditorMode = 'style' | 'frame'

export function AppearanceEditor({ mode }: { mode: AppearanceEditorMode }) {
  const { t } = useTranslation('site')
  const catalog = useQuery({ queryKey: ['site', 'appearance-catalog'], queryFn: () => api.get<Catalog>('/v1/site/appearance') })
  const [selection, setSelection] = useState('site')
  const [visited, setVisited] = useState<string[]>(['site'])
  if (catalog.isPending) return <Skeleton className="h-48" />
  if (!catalog.data) return <ErrorState onRetry={() => void catalog.refetch()} />
  if (!catalog.data.available) return <p>{t('appearanceEditor.unavailable')}</p>
  const targets = mode === 'frame' ? catalog.data.targets.filter((item) => item.entity === 'site.appearance') : catalog.data.targets
  const target = targets.find((item) => item.target === selection) ?? targets[0]
  if (!target) return null
  return <UnsavedChangesScope><div className="grid gap-6">
    {targets.length > 1 && <Field label={t('appearanceEditor.scope')}>{(control) => <NativeSelect {...control} className="w-full md:max-w-sm" value={target.target} onValueChange={(next) => { setVisited((current) => current.includes(next) ? current : [...current, next]); setSelection(next) }}>
      {targets.map((item) => <option key={item.target} value={item.target}>{item.label ?? t('appearanceEditor.global')}</option>)}
    </NativeSelect>}</Field>}
    {targets.filter((item) => visited.includes(item.target) || item.target === target.target).map((item) => <div key={item.target} hidden={item.target !== target.target}><TargetEditor catalog={catalog.data!} target={item} mode={mode} /></div>)}
  </div></UnsavedChangesScope>
}

function TargetEditor({ catalog, target, mode }: { catalog: Catalog; target: Target; mode: AppearanceEditorMode }) {
  const [generation, setGeneration] = useState(0)
  const record = useQuery({ queryKey: ['site', 'appearance', target.target], queryFn: async () => (await api.get<{ data: AppearanceRecord[] }>(recordUrl(target.entity), { query: { 'filter[target][eq]': target.target, per_page: '1' } })).data[0] ?? null })
  if (record.isPending) return <Skeleton className="h-48" />
  if (record.isError && record.data === undefined) return <ErrorState onRetry={() => void record.refetch()} />
  return <AppearanceForm key={generation} catalog={catalog} target={target} mode={mode} record={record.data ?? null} onReload={async () => { const result = await record.refetch(); if (result.error) throw result.error; setGeneration((current) => current + 1) }} />
}

function AppearanceForm({ catalog, target, mode, record, onReload }: { catalog: Catalog; target: Target; mode: AppearanceEditorMode; record: AppearanceRecord | null; onReload: () => Promise<void> }) {
  const { t } = useTranslation('site')
  const client = useQueryClient()
  const configuration = { data: catalog.frame.configuration }
  const overview = { data: catalog.frame.overview }
  const [values, setValues] = useState<Values>(() => parsed(record?.draft_values))
  const [files, setFiles] = useState<Values>(() => ({ logo: record?.draft_logo ?? null, dark_logo: record?.draft_dark_logo ?? null, icon: record?.draft_icon ?? null }))
  const [busy, setBusy] = useState(false)
  const [reloadRequested, setReloadRequested] = useState(false)
  const [html, setHtml] = useState('')
  const [path, setPath] = useState(target.path)
  const [mobile, setMobile] = useState(false)
  const [empty, setEmpty] = useState(false)
  const [saved, setSaved] = useState(record)
  const labels = t('appearanceEditor.properties', { returnObjects: true }) as Record<string, string>
  const options = t('appearanceEditor.options', { returnObjects: true }) as Record<string, string>
  const groups = t('appearanceEditor.groups', { returnObjects: true }) as Record<string, string>
  const theme = Object.hasOwn(values, 'theme') ? catalog.theme_catalogs[String(values.theme)] : undefined
  const themeProperties = theme?.properties ?? catalog.theme_properties
  const themeDefaults = theme?.defaults ?? catalog.theme_defaults
  const style = mode === 'style'
  const frameKeys = new Set([...catalog.properties.filter((property) => property.group === 'frame').map((property) => property.key), 'tagline'])
  const ownKey = (key: string) => frameKeys.has(key) !== style
  const merge = (base: Values, replacement: Values): Values => ({ ...Object.fromEntries(Object.entries(base).filter(([key]) => !ownKey(key))), ...Object.fromEntries(Object.entries(replacement).filter(([key]) => ownKey(key))) })
  const properties = catalog.properties.filter((property) => property.kind !== 'styles' && property.key !== 'variation' && (property.group === 'frame') !== style && (property.key === 'theme' || themeProperties.length === 0 || themeProperties.includes(property.key)))
  const unsupported = Object.keys(values).some((key) => key !== 'theme' && themeProperties.length > 0 && !themeProperties.includes(key))
  const dirty = JSON.stringify(values) !== JSON.stringify(parsed(saved?.draft_values)) || Object.entries(files).some(([key, value]) => value !== (saved?.[`draft_${key}` as keyof AppearanceRecord] ?? null))
  const change = (key: string, value: string | boolean | null) => setValues((current) => ({ ...current, [key]: value }))
  const inherit = (key: string) => setValues((current) => { const next = { ...current }; delete next[key]; return next })
  const variationOf = (key: unknown) => catalog.variations[String(key ?? 'default')] ?? {}
  const variationKey = values.variation ?? themeDefaults.variation ?? 'default'
  const themeStyles = mergeStyles(catalog.theme_styles ?? {}, (themeDefaults.styles as Values | undefined) ?? {}) as Values
  const styleBase = mergeStyles(mergeStyles(catalog.base_styles, themeStyles), variationOf(variationKey)) as Values
  const presetStyles = (key: string) => mergeStyles(mergeStyles(catalog.base_styles, themeStyles), variationOf(key)) as Values

  const collectors = useRef(new Map<string, FrameCollector>())
  const registerCollector = useCallback((id: string, collector: FrameCollector | null) => {
    if (collector) collectors.current.set(id, collector)
    else collectors.current.delete(id)
  }, [])
  function withFrames(): Values | null {
    let next: Values = values
    for (const collect of collectors.current.values()) {
      const part = collect()
      if (part === null) return null
      next = { ...next, ...part }
    }
    return next
  }
  async function saveAll(publish: boolean) {
    const next = withFrames()
    if (!next) {
      notifyError(t('appearanceEditor.frameInvalid'))
      return
    }
    if (next !== values) setValues(next)
    if (publish) await apply(next)
    else {
      await save(next)
      notifySuccess(t('appearanceEditor.saved'))
    }
  }
  async function save(nextValues = values, nextFiles = files) {
    const payload = { target: target.target, draft_values: JSON.stringify(nextValues), ...Object.fromEntries(Object.entries(nextFiles).map(([key, value]) => [`draft_${key}`, value])) }
    const result = saved ? await api.patch<AppearanceRecord>(`${recordUrl(target.entity)}/${saved.id}`, { ...payload, _expected_updated_at: saved.updated_at }) : await api.post<AppearanceRecord>(recordUrl(target.entity), payload)
    setSaved(result)
    return result
  }
  async function run(action: () => Promise<void>) {
    setBusy(true)
    try { await action() } catch (error) { notifyError(error) } finally { setBusy(false) }
  }
  async function apply(nextValues = values) {
    const current = await save(nextValues)
    const revision = await api.post<{ id: string }>('/v1/publication/revisions', { record_type: target.entity, record_id: current.id, expected_version: current.updated_at })
    const request = await api.post<{ status: string; reason_code: string | null }>('/v1/publication/requests', { revision_id: revision.id, idempotency_key: crypto.randomUUID() })
    if (['blocked', 'failed', 'cancelled', 'superseded'].includes(request.status)) {
      notifyError(t(request.reason_code === 'publication_approvals_unavailable' ? 'appearanceEditor.approvalsUnavailable' : 'appearanceEditor.publicationFailed'))
    } else {
      notifySuccess(request.status === 'completed' ? t('appearanceEditor.applied') : t('appearanceEditor.pending'))
    }
    const latest = await api.get<AppearanceRecord>(`${recordUrl(target.entity)}/${current.id}`)
    setSaved(latest)
    await client.invalidateQueries({ queryKey: ['site', 'appearance', target.target] })
    await client.invalidateQueries({ queryKey: ['site', 'appearance-catalog'] })
  }
  async function preview() {
    const previewValues = { ...values }
    const response = await api.post<{ html: string }>('/v1/site/appearance/preview', { entity: target.entity, target: target.target, values: previewValues, path, files, empty })
    setHtml(response.html)
  }
  return <div className="grid gap-6">
    <UnsavedChangesGuard when={dirty} />
    <UnsavedChangesDialog open={reloadRequested} onStay={() => setReloadRequested(false)} onLeave={() => { setReloadRequested(false); void run(onReload) }} />
    {record && record.updated_at > (saved?.updated_at ?? '') && <div role="status"><p>{t('appearanceEditor.remoteChange')}</p><Button variant="outline" disabled={busy} onClick={() => setReloadRequested(true)}>{t('appearanceEditor.reload')}</Button></div>}
    <p className="text-sm text-muted-foreground">{!style ? t('appearanceEditor.frameHelp') : t('appearanceEditor.globalHelp')}</p>
    {style && unsupported && <p role="status">{t('appearanceEditor.unsupported')}</p>}
    {style && <ThemePresets labels={Object.fromEntries(catalog.theme_variations.map((variation) => [variation.key, variation.name]))} presets={catalog.presets.map((preset) => ({ ...preset, values: preset.values, styles: presetStyles(preset.key) }))} fontStacks={catalog.font_stacks} active={String(variationKey)} values={values} disabled={busy} onApply={(preset) => setValues((current) => { const next = { ...current, ...preset.values }; delete next.styles; return next })} />}
    {style && <StylesEditor base={styleBase} overrides={(values.styles as Values | undefined) ?? {}} fontStacks={catalog.font_stacks} disabled={busy} onChange={(styles) => setValues((current) => { const next = { ...current }; if (styles) next.styles = styles; else delete next.styles; return next })} />}
    {[...new Set(properties.map((property) => property.group))].map((group) => <Card key={group}>
      <CardHeader><CardTitle>{groups[group]}</CardTitle></CardHeader>
      <CardContent className="grid gap-5 md:grid-cols-2">
        {properties.filter((property) => property.group === group && property.kind !== 'frame' && property.key !== 'tagline').map((property) => {
          if (property.kind === 'file') return saved ? <BlockImageField key={property.key} label={labels[property.key]} record={{ recordType: target.entity, recordId: saved.id }} value={files[property.key] as string | null} onChange={(value) => setFiles((current) => ({ ...current, [property.key]: value }))} disabled={busy} /> : <p key={property.key} className="text-sm text-muted-foreground">{t('appearanceEditor.saveForImages')}</p>
          return <Field key={property.key} label={labels[property.key]} description={property.key === 'name' ? t('appearanceEditor.nameHelp') : undefined} labelAction={Object.hasOwn(values, property.key) ? <Button size="sm" variant="ghost" disabled={busy} onClick={() => inherit(property.key)}>{t('appearanceEditor.inherit')}</Button> : null}>
            {(control) => property.kind === 'boolean' ? <NativeSelect {...control} className="w-full" disabled={busy} value={Object.hasOwn(values, property.key) ? String(values[property.key]) : ''} onValueChange={(next) => next ? change(property.key, next === 'true') : inherit(property.key)}>
              <option value="">{t('appearanceEditor.inherited')}</option><option value="true">{t('appearanceEditor.yes')}</option><option value="false">{t('appearanceEditor.no')}</option>
            </NativeSelect> : property.kind === 'select' ? <NativeSelect {...control} className="w-full" disabled={busy} value={property.key === 'theme' && values.theme === '' ? '__default__' : String(values[property.key] ?? '')} onValueChange={(next) => next ? change(property.key, next === '__default__' ? '' : next) : inherit(property.key)}>
              <option value="">{t('appearanceEditor.inherited')}</option>{property.options.map((option) => <option key={option} value={option === '' ? '__default__' : option}>{option === '' ? t('appearance.defaultTheme') : catalog.themes.find((theme) => theme.key === option)?.name ?? options[option] ?? option}</option>)}
            </NativeSelect> : <Input {...control} type={property.kind === 'color' ? 'color' : 'text'} disabled={busy} maxLength={120} value={String(values[property.key] ?? themeDefaults[property.key] ?? property.default ?? '')} onChange={(event) => change(property.key, event.target.value)} />}
          </Field>
        })}
      </CardContent>
    </Card>)}
    {!style && configuration.data && overview.data && <AppearanceFrameCollect.Provider value={registerCollector}><AppearanceFrameDraft.Provider value={async (frame) => { const next = { ...values, ...frame }; await save(next); setValues(next) }}>
      <HeaderCard key={JSON.stringify([values.header, values.tagline, values.translations])} configuration={{ ...configuration.data, site: { ...configuration.data.site, ...values } as SiteModuleSettings }} overview={overview.data} />
      <FooterCard key={JSON.stringify([values.footer, values.translations])} configuration={{ ...configuration.data, site: { ...configuration.data.site, ...values } as SiteModuleSettings }} overview={overview.data} />
    </AppearanceFrameDraft.Provider></AppearanceFrameCollect.Provider>}
    <div className="flex flex-wrap gap-2">
      <Button variant="outline" disabled={busy} onClick={() => void run(() => saveAll(false))}>{t('appearanceEditor.save')}</Button>
      <Button disabled={busy} onClick={() => void run(() => saveAll(true))}>{t('appearanceEditor.apply')}</Button>
      <Button variant="outline" disabled={busy} onClick={() => void run(async () => { const next = merge(values, parsed(saved?.values)); const nextFiles = style ? { logo: saved?.logo ?? null, dark_logo: saved?.dark_logo ?? null, icon: saved?.icon ?? null } : files; await save(next, nextFiles); setValues(next); setFiles(nextFiles) })}>{t('appearanceEditor.discard')}</Button>
      <Button variant="outline" disabled={busy} onClick={() => { setValues(merge(values, {})); if (style) setFiles({ logo: null, dark_logo: null, icon: null }) }}>{t('appearanceEditor.defaults')}</Button>
    </div>
    <Card><CardHeader><CardTitle>{t('appearanceEditor.preview')}</CardTitle></CardHeader><CardContent className="grid gap-4">
      <Field label={t('appearanceEditor.path')} description={t('appearanceEditor.pathHelp')}>{(control) => <Input {...control} value={path} onChange={(event) => setPath(event.target.value)} />}</Field>
      {style && <ToggleField label={t('appearanceEditor.emptyPreview')} checked={empty} onCheckedChange={(checked) => setEmpty(checked === true)} />}
      <div className="flex gap-2"><Button variant="outline" disabled={busy} onClick={() => void run(preview)}>{t('appearanceEditor.preview')}</Button><Button variant="outline" onClick={() => setMobile(!mobile)}>{mobile ? t('appearanceEditor.desktop') : t('appearanceEditor.mobile')}</Button></div>
      {html && <iframe title={t('appearanceEditor.preview')} sandbox="" srcDoc={html} className="mx-auto h-[42rem] max-w-full border" style={{ width: mobile ? 390 : '100%' }} />}
    </CardContent></Card>
  </div>
}
