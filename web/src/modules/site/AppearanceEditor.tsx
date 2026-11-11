import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { api } from '@/api'
import { notifyError, notifySuccess, UnsavedChangesGuard } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { BlockImageField } from '@/sdk'
import { AppearanceFrameDraft, type SiteModuleSettings } from './siteSettings'
import { HeaderCard, FooterCard } from './SiteFrameCards'

type Values = Record<string, unknown>
interface Property { key: string; group: string; kind: string; default: string | boolean | null; options: string[]; collection: boolean }
interface Target { entity: string; target: string; label: string | null; path: string }
interface Catalog { frame: { configuration: { site: SiteModuleSettings }; overview: { base_url: string; default_language: string } };  theme_properties: string[]; theme_defaults: Values; themes: { key: string; name: string }[]; available: boolean; properties: Property[]; presets: { key: string; values: Values }[]; targets: Target[] }
interface AppearanceRecord { id: string; draft_values?: string | null; values?: string | null; updated_at: string; draft_logo?: string | null; draft_dark_logo?: string | null; draft_icon?: string | null; draft_opening_image?: string | null; opening_image?: string | null; logo?: string | null; dark_logo?: string | null; icon?: string | null }
const recordUrl = (entity: string) => `/v1/entities/${entity}/records`
const parsed = (text?: string | null): Values => text ? JSON.parse(text) as Values : {}

export function AppearanceEditor() {
  const { t } = useTranslation('site')
  const catalog = useQuery({ queryKey: ['site', 'appearance-catalog'], queryFn: () => api.get<Catalog>('/v1/site/appearance') })
  const [selection, setSelection] = useState('site')
  if (catalog.isPending) return <Skeleton className="h-48" />
  if (!catalog.data) return <ErrorState onRetry={() => void catalog.refetch()} />
  if (!catalog.data.available) return <p>{t('appearanceEditor.unavailable')}</p>
  const target = catalog.data.targets.find((item) => item.target === selection) ?? catalog.data.targets[0]
  if (!target) return null
  return <div className="grid gap-6">
    <Field label={t('appearanceEditor.scope')}>{(control) => <select {...control} className="h-10 rounded-md border bg-background px-3" value={target.target} onChange={(event) => setSelection(event.target.value)}>
      {catalog.data.targets.map((item) => <option key={item.target} value={item.target}>{item.label ?? t('appearanceEditor.global')}</option>)}
    </select>}</Field>
    <TargetEditor key={target.target} catalog={catalog.data} target={target} />
  </div>
}

function TargetEditor({ catalog, target }: { catalog: Catalog; target: Target }) {
  const record = useQuery({ queryKey: ['site', 'appearance', target.target], queryFn: async () => (await api.get<{ data: AppearanceRecord[] }>(recordUrl(target.entity), { query: { 'filter[target][eq]': target.target, per_page: '1' } })).data[0] ?? null })
  if (record.isPending) return <Skeleton className="h-48" />
  if (record.isError) return <ErrorState onRetry={() => void record.refetch()} />
  return <AppearanceForm key={record.data?.updated_at ?? 'new'} catalog={catalog} target={target} record={record.data ?? null} />
}

function AppearanceForm({ catalog, target, record }: { catalog: Catalog; target: Target; record: AppearanceRecord | null }) {
  const { t } = useTranslation('site')
  const client = useQueryClient()
  const configuration = { data: catalog.frame.configuration }
  const overview = { data: catalog.frame.overview }
  const [values, setValues] = useState<Values>(() => parsed(record?.draft_values))
  const [files, setFiles] = useState<Values>(() => ({ logo: record?.draft_logo ?? null, dark_logo: record?.draft_dark_logo ?? null, icon: record?.draft_icon ?? null, opening_image: record?.draft_opening_image ?? null }))
  const [busy, setBusy] = useState(false)
  const [html, setHtml] = useState('')
  const [path, setPath] = useState(target.path)
  const [mobile, setMobile] = useState(false)
  const [empty, setEmpty] = useState(false)
  const [saved, setSaved] = useState(record)
  const labels = t('appearanceEditor.properties', { returnObjects: true }) as Record<string, string>
  const options = t('appearanceEditor.options', { returnObjects: true }) as Record<string, string>
  const groups = t('appearanceEditor.groups', { returnObjects: true }) as Record<string, string>
  const presets = t('appearanceEditor.presets', { returnObjects: true }) as Record<string, string>
  const local = target.entity === 'site.presentation'
  const properties = catalog.properties.filter((property) => (!local || property.collection) && (property.key === 'theme' || catalog.theme_properties.length === 0 || catalog.theme_properties.includes(property.key)))
  const unsupported = Object.keys(values).some((key) => key !== 'theme' && catalog.theme_properties.length > 0 && !catalog.theme_properties.includes(key))
  const dirty = JSON.stringify(values) !== JSON.stringify(parsed(saved?.draft_values)) || Object.entries(files).some(([key, value]) => value !== (saved?.[`draft_${key}` as keyof AppearanceRecord] ?? null))
  const change = (key: string, value: string | boolean | null) => setValues((current) => ({ ...current, [key]: value }))
  const inherit = (key: string) => setValues((current) => { const next = { ...current }; delete next[key]; return next })
  const color = (key: string) => String(values[key] ?? catalog.properties.find((property) => property.key === key)?.default ?? '#ffffff')
  const luminance = (hex: string) => { const rgb = [1, 3, 5].map((start) => parseInt(hex.slice(start, start + 2), 16) / 255).map((value) => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4); return 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2] }
  const contrast = (foreground: string, background: string) => { const a = luminance(color(foreground)); const b = luminance(color(background)); return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05) }
  const lowContrast = !local && ['text', 'muted', 'link'].some((key) => contrast(key, 'background') < 4.5)

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
  async function apply() {
    const current = await save()
    const revision = await api.post<{ id: string }>('/v1/publication/revisions', { record_type: target.entity, record_id: current.id, expected_version: current.updated_at })
    const request = await api.post<{ status: string }>('/v1/publication/requests', { revision_id: revision.id, idempotency_key: crypto.randomUUID() })
    notifySuccess(request.status === 'completed' ? t('appearanceEditor.applied') : t('appearanceEditor.pending'))
    await client.invalidateQueries({ queryKey: ['site', 'appearance', target.target] })
  }
  async function preview() {
    const previewValues = { ...values }
    const response = await api.post<{ html: string }>('/v1/site/appearance/preview', { entity: target.entity, target: target.target, values: previewValues, path, files, empty })
    setHtml(response.html)
  }
  return <div className="grid gap-6">
    <UnsavedChangesGuard when={dirty} />
    <p className="text-sm text-muted-foreground">{local ? t('appearanceEditor.collectionHelp') : t('appearanceEditor.globalHelp')}</p>
    {unsupported && <p role="status">{t('appearanceEditor.unsupported')}</p>}
    {lowContrast && <p role="status" className="text-sm text-destructive">{t('appearanceEditor.contrast')}</p>}
    <div className="flex flex-wrap gap-2">
      {catalog.presets.map((preset) => <Button key={preset.key} variant="outline" disabled={busy} onClick={() => setValues((current) => ({ ...current, ...preset.values }))}>{presets[preset.key]}</Button>)}
    </div>
    {[...new Set(properties.map((property) => property.group))].map((group) => <Card key={group}>
      <CardHeader><CardTitle>{groups[group]}</CardTitle></CardHeader>
      <CardContent className="grid gap-5 md:grid-cols-2">
        {properties.filter((property) => property.group === group && property.kind !== 'frame' && property.key !== 'tagline').map((property) => {
          if (property.kind === 'file') return saved ? <BlockImageField key={property.key} label={labels[property.key]} record={{ recordType: target.entity, recordId: saved.id }} value={files[property.key] as string | null} onChange={(value) => setFiles((current) => ({ ...current, [property.key]: value }))} disabled={busy} /> : <p key={property.key} className="text-sm text-muted-foreground">{t('appearanceEditor.saveForImages')}</p>
          return <Field key={property.key} label={labels[property.key]} labelAction={Object.hasOwn(values, property.key) ? <Button size="sm" variant="ghost" disabled={busy} onClick={() => inherit(property.key)}>{t('appearanceEditor.inherit')}</Button> : null}>
            {(control) => property.kind === 'boolean' ? <select {...control} className="h-10 rounded-md border bg-background px-3" disabled={busy} value={Object.hasOwn(values, property.key) ? String(values[property.key]) : ''} onChange={(event) => event.target.value ? change(property.key, event.target.value === 'true') : inherit(property.key)}>
              <option value="">{t('appearanceEditor.inherited')}</option><option value="true">{t('appearanceEditor.yes')}</option><option value="false">{t('appearanceEditor.no')}</option>
            </select> : property.kind === 'select' ? <select {...control} className="h-10 rounded-md border bg-background px-3" disabled={busy} value={property.key === 'theme' && values.theme === '' ? '__default__' : String(values[property.key] ?? '')} onChange={(event) => event.target.value ? change(property.key, event.target.value === '__default__' ? '' : event.target.value) : inherit(property.key)}>
              <option value="">{t('appearanceEditor.inherited')}</option>{property.options.map((option) => <option key={option} value={option === '' ? '__default__' : option}>{option === '' ? t('appearance.defaultTheme') : catalog.themes.find((theme) => theme.key === option)?.name ?? options[option] ?? option}</option>)}
            </select> : <Input {...control} type={property.kind === 'color' ? 'color' : 'text'} disabled={busy} maxLength={120} value={String(values[property.key] ?? property.default ?? '')} onChange={(event) => change(property.key, event.target.value)} />}
          </Field>
        })}
      </CardContent>
    </Card>)}
    {!local && configuration.data && overview.data && <AppearanceFrameDraft.Provider value={async (frame) => { const next = { ...values, ...frame }; await save(next); setValues(next) }}>
      <HeaderCard key={JSON.stringify([values.header, values.tagline, values.translations])} configuration={{ ...configuration.data, site: { ...configuration.data.site, ...values } as SiteModuleSettings }} overview={overview.data} />
      <FooterCard key={JSON.stringify([values.footer, values.translations])} configuration={{ ...configuration.data, site: { ...configuration.data.site, ...values } as SiteModuleSettings }} overview={overview.data} />
    </AppearanceFrameDraft.Provider>}
    <div className="flex flex-wrap gap-2">
      <Button disabled={busy} onClick={() => void run(async () => { await save(); notifySuccess(t('appearanceEditor.saved')) })}>{t('appearanceEditor.save')}</Button>
      <Button disabled={busy} onClick={() => void run(apply)}>{t('appearanceEditor.apply')}</Button>
      <Button variant="outline" disabled={busy} onClick={() => void run(async () => { const next = parsed(saved?.values); const nextFiles = { logo: saved?.logo ?? null, dark_logo: saved?.dark_logo ?? null, icon: saved?.icon ?? null, opening_image: saved?.opening_image ?? null }; await save(next, nextFiles); setValues(next); setFiles(nextFiles) })}>{t('appearanceEditor.discard')}</Button>
      <Button variant="outline" disabled={busy} onClick={() => { setValues({}); setFiles({ logo: null, dark_logo: null, icon: null, opening_image: null }) }}>{t('appearanceEditor.defaults')}</Button>
    </div>
    <Card><CardHeader><CardTitle>{t('appearanceEditor.preview')}</CardTitle></CardHeader><CardContent className="grid gap-4">
      <Field label={t('appearanceEditor.path')} description={t('appearanceEditor.pathHelp')}>{(control) => <Input {...control} value={path} onChange={(event) => setPath(event.target.value)} />}</Field>
      <Field label={t('appearanceEditor.emptyPreview')}>{(control) => <input {...control} type="checkbox" checked={empty} onChange={(event) => setEmpty(event.target.checked)} />}</Field>
      <div className="flex gap-2"><Button variant="outline" disabled={busy} onClick={() => void run(preview)}>{t('appearanceEditor.preview')}</Button><Button variant="outline" onClick={() => setMobile(!mobile)}>{mobile ? t('appearanceEditor.desktop') : t('appearanceEditor.mobile')}</Button></div>
      {html && <iframe title={t('appearanceEditor.preview')} sandbox="" srcDoc={html} className="mx-auto h-[42rem] max-w-full border" style={{ width: mobile ? 390 : '100%' }} />}
    </CardContent></Card>
  </div>
}
