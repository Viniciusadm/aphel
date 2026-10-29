import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { api, isApiError, type components } from '@/api'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { NativeSelect } from '@/features/settings/organization/components/NativeSelect'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { fieldViolations } from '@/features/auth/lib/api-errors'

export type InstallerModule = components['schemas']['InstallerModuleResponse']
export type InstallerCatalog = components['schemas']['InstallerStatusResponse']
export type ModuleSettings = Record<string, Record<string, unknown>>
export interface ModuleChoice { modules: string[]; module_settings: ModuleSettings }

function dependencyClosure(catalog: InstallerModule[], selected: string[]): Set<string> {
  const keys = new Set([...selected, ...catalog.filter((item) => item.required).map((item) => item.key)])
  let previous = -1
  while (previous !== keys.size) {
    previous = keys.size
    for (const item of catalog) if (keys.has(item.key)) for (const key of item.dependencies) keys.add(key)
  }
  return keys
}

function needsConfiguration(item: InstallerModule, catalog: InstallerModule[]): boolean {
  const closure = dependencyClosure(catalog, [item.key])
  return catalog.some((module) => closure.has(module.key) && !module.required && module.settings_schema.some((field) => field.required))
}

function ModuleName({ module }: { module: InstallerModule }) {
  const translate = useModuleTranslation(module.key)
  const name = translate('name')
  return <>{name === 'name' ? module.name : name}</>
}

function ModuleCard({ module, checked, locked, reason, onToggle, onConfigure }: {
  module: InstallerModule; checked: boolean; locked: boolean; reason: string; onToggle: () => void; onConfigure: () => void
}) {
  const { t } = useTranslation('installer')
  const translate = useModuleTranslation(module.key)
  const description = translate('description')
  return (
    <div className="grid gap-2 rounded-lg border p-4">
      <label className="flex items-start gap-3 font-medium">
        <input type="checkbox" className="mt-1 size-4" checked={checked} disabled={locked || !module.compatible} onChange={onToggle} />
        <ModuleName module={module} />
      </label>
      <p className="text-sm text-muted-foreground">{description === 'description' ? module.description : description}</p>
      {reason && <p className="text-xs text-muted-foreground">{reason}</p>}
      {!module.compatible && <p className="text-xs text-destructive">{t('selection.incompatible')}</p>}
      {checked && !module.required && module.settings_schema.length > 0 && <Button type="button" variant="outline" size="sm" onClick={onConfigure}>{t('selection.configure')}</Button>}
    </div>
  )
}

export function ModuleSelection({ catalog, value, onChange, stage, profile, onProfileChange }: {
  catalog: InstallerCatalog; value: ModuleChoice; onChange: (value: ModuleChoice) => void; stage: 'profiles' | 'shelf' | 'review'; profile: string; onProfileChange: (key: string) => void
}) {
  const { t } = useTranslation('installer')
  const [search, setSearch] = useState('')
  const [showAll, setShowAll] = useState(false)
  const [editing, setEditing] = useState<string | null>(null)
  const selected = dependencyClosure(catalog.modules, value.modules)
  const chosenProfile = catalog.profiles.find((item) => item.key === profile)
  const recommended = new Set(chosenProfile?.modules ?? [])
  const isShelf = (item: InstallerModule) => needsConfiguration(item, catalog.modules)
  const choose = (key: string) => {
    const preset = catalog.profiles.find((item) => item.key === key)
    onProfileChange(key)
    onChange({ modules: catalog.modules.filter((item) => !item.required && item.compatible && preset?.modules.includes(item.key) && !isShelf(item)).map((item) => item.key), module_settings: {} })
  }
  const remove = (key: string) => {
    const modules = value.modules.filter((item) => item !== key)
    const remaining = dependencyClosure(catalog.modules, modules)
    onChange({ modules, module_settings: Object.fromEntries(Object.entries(value.module_settings).filter(([item]) => remaining.has(item))) })
  }
  const modules = catalog.modules.filter((item) => !item.required && (stage === 'review' ? selected.has(item.key) : stage === 'shelf' ? isShelf(item) && (showAll || !chosenProfile || recommended.has(item.key)) : !isShelf(item)))
  const personalized = chosenProfile && (value.modules.some((key) => !recommended.has(key)) || catalog.modules.some((item) => !item.required && item.compatible && recommended.has(item.key) && !isShelf(item) && !selected.has(item.key)))
  return (
    <section className="grid gap-4">
      <h2 className="text-lg font-semibold">{t(`selection.${stage}`)}</h2>
      <p className="text-sm text-muted-foreground">{t(`selection.${stage}Hint`)}</p>
      {stage === 'profiles' && <>
        <div className="grid gap-2 sm:grid-cols-2">
          <Button type="button" variant={!profile ? 'default' : 'outline'} onClick={() => choose('')}>{t('selection.custom')}</Button>
          {catalog.profiles.map((item) => <button key={item.key} type="button" aria-pressed={profile === item.key} className={`grid gap-1 rounded-lg border p-4 text-left transition-colors hover:bg-muted ${profile === item.key ? 'border-primary bg-primary/5' : ''}`} onClick={() => choose(item.key)}><span className="font-medium">{t(`selection.profilesNames.${item.key}` as 'selection.profilesNames.freelancer')}</span><span className="text-sm text-muted-foreground">{t(`selection.profilesDescriptions.${item.key}` as 'selection.profilesDescriptions.freelancer')}</span></button>)}
        </div>
        {personalized && <p role="status" className="text-sm">{t('selection.personalized')}</p>}
        {chosenProfile && chosenProfile.modules.some((key) => !catalog.modules.some((item) => item.key === key && item.compatible)) && <p role="status" className="text-sm">{t('selection.partial')}</p>}
        <Button type="button" variant="ghost" onClick={() => choose('')}>{t('selection.clear')}</Button>
      </>}
      {stage === 'shelf' && chosenProfile && <label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={showAll} onChange={(event) => setShowAll(event.target.checked)} />{t('selection.showAll')}</label>}
      {stage !== 'review' && <Input aria-label={t('selection.search')} placeholder={t('selection.search')} value={search} onChange={(event) => setSearch(event.target.value)} />}
      <div className="grid gap-3 sm:grid-cols-2">
        {modules.map((item) => <SearchableModule key={item.key} module={item} search={stage === 'review' ? '' : search}>
          <ModuleCard module={item} checked={selected.has(item.key)} locked={stage === 'review' || (selected.has(item.key) && !value.modules.includes(item.key)) || catalog.modules.some((other) => other.key !== item.key && selected.has(other.key) && other.dependencies.includes(item.key))}
            reason={selected.has(item.key) && !value.modules.includes(item.key) ? t('selection.dependency') : stage === 'shelf' && recommended.has(item.key) ? t('selection.recommended') : ''}
            onToggle={() => selected.has(item.key) ? remove(item.key) : isShelf(item) ? setEditing(item.key) : onChange({ ...value, modules: [...value.modules, item.key] })} onConfigure={() => setEditing(item.key)} />
        </SearchableModule>)}
      </div>
      {stage === 'review' && value.modules.length === 0 && <p>{t('selection.none')}</p>}
      {stage !== 'shelf' && <details><summary className="cursor-pointer text-sm font-medium">{t('selection.required')}</summary><ul className="mt-2 grid gap-1 text-sm text-muted-foreground">{catalog.modules.filter((item) => item.required).map((item) => <li key={item.key}><ModuleName module={item} /></li>)}</ul></details>}
      {editing && <Configuration key={editing} catalog={catalog} moduleKey={editing} value={value} onCancel={() => setEditing(null)} onSave={(next) => { onChange(next); setEditing(null) }} />}
    </section>
  )
}

function SearchableModule({ module, search, children }: { module: InstallerModule; search: string; children: React.ReactNode }) {
  const translate = useModuleTranslation(module.key)
  const match = `${module.key} ${module.name} ${translate('name')} ${translate('description')}`.toLocaleLowerCase().includes(search.toLocaleLowerCase())
  return match ? <>{children}</> : null
}

function Configuration({ catalog, moduleKey, value, onSave, onCancel }: { catalog: InstallerCatalog; moduleKey: string; value: ModuleChoice; onSave: (value: ModuleChoice) => void; onCancel: () => void }) {
  const { t } = useTranslation('installer')
  const closure = dependencyClosure(catalog.modules, [moduleKey])
  const items = catalog.modules.filter((item) => !item.required && closure.has(item.key) && item.settings_schema.length > 0)
  const [settings, setSettings] = useState<ModuleSettings>(() => Object.fromEntries(items.map((item) => [item.key, { ...Object.fromEntries(item.settings_schema.filter((field) => field.type !== 'secret' && field.default != null).map((field) => [field.key, field.default])), ...value.module_settings[item.key] }])))
  const [error, setError] = useState<string | null>(null)
  const [violations, setViolations] = useState<string[]>([])
  const [saving, setSaving] = useState(false)
  const save = async () => {
    setSaving(true)
    setError(null)
    setViolations([])
    const next = { modules: dependencyClosure(catalog.modules, value.modules).has(moduleKey) ? value.modules : [...value.modules, moduleKey], module_settings: { ...value.module_settings, ...settings } }
    try {
      await api.post('/v1/installer/modules/validate', next)
      onSave(next)
    } catch (failure) {
      setError(t('selection.invalidConfiguration'))
      if (isApiError(failure)) setViolations(fieldViolations(failure.details).map((item) => item.field))
    } finally { setSaving(false) }
  }
  return <Dialog open onOpenChange={(open) => !open && !saving && onCancel()}><DialogContent className="max-h-[85vh] overflow-y-auto"><DialogHeader><DialogTitle>{t('selection.configure')}</DialogTitle><DialogDescription>{t('selection.configurationHint')}</DialogDescription></DialogHeader>
    <form className="grid gap-4" onSubmit={(event) => { event.preventDefault(); event.stopPropagation(); void save() }}>
      {error && <p role="alert" className="text-sm text-destructive">{error}</p>}
      <fieldset disabled={saving} className="grid gap-5">{items.map((item) => <ConfigurationFields key={item.key} module={item} values={settings[item.key] ?? {}} violations={violations} onChange={(key, fieldValue) => setSettings((current) => ({ ...current, [item.key]: { ...current[item.key], [key]: fieldValue } }))} />)}</fieldset>
      <div className="flex gap-2"><Button type="button" variant="outline" disabled={saving} onClick={onCancel}>{t('selection.cancel')}</Button><Button type="submit" disabled={saving}>{saving ? t('selection.validating') : t('selection.add')}</Button></div>
    </form>
  </DialogContent></Dialog>
}

function ConfigurationFields({ module, values, violations, onChange }: { module: InstallerModule; values: Record<string, unknown>; violations: string[]; onChange: (key: string, value: unknown) => void }) {
  const translate = useModuleTranslation(module.key)
  const { t } = useTranslation('installer')
  const lookup = (key: string, fallback: string) => { const result = translate(key); return result === key ? fallback : result }
  return <div className="grid gap-3"><h3 className="font-semibold"><ModuleName module={module} /></h3>{module.settings_schema.map((field) => {
    const id = `installer-setting-${module.key}-${field.key}`
    const invalid = violations.some((path) => path === `module_settings.${module.key}.settings.${field.key}` || path === `module_settings.${module.key}.${field.key}`)
    const help = lookup(`settings.${field.key}.help`, field.help ?? '')
    return <div key={field.key} className="grid gap-1"><label htmlFor={id} className="text-sm font-medium">{lookup(`settings.${field.key}.label`, field.label)}{field.required ? ' *' : ''}</label>
      {field.type === 'boolean' ? <input id={id} type="checkbox" checked={values[field.key] === true} onChange={(event) => onChange(field.key, event.target.checked)} /> : field.type === 'select' ? <NativeSelect id={id} required={field.required} value={String(values[field.key] ?? '')} onChange={(event) => onChange(field.key, event.target.value)}><option value="">{t('selection.selectOption')}</option>{field.options.map((option) => <option key={option.value} value={option.value}>{lookup(`settings.${field.key}.options.${option.value}`, option.label)}</option>)}</NativeSelect> : <Input id={id} type={field.type === 'secret' ? 'password' : field.type === 'number' ? 'number' : field.type === 'email' ? 'email' : field.type === 'url' ? 'url' : 'text'} required={field.required} autoComplete={field.type === 'secret' ? 'new-password' : 'off'} min={field.min ?? undefined} max={field.max ?? undefined} step="any" maxLength={field.type === 'secret' ? 4096 : field.type === 'long_text' ? 20000 : 500} aria-invalid={invalid || undefined} aria-describedby={help ? `${id}-help` : undefined} value={String(values[field.key] ?? '')} onChange={(event) => onChange(field.key, field.type === 'number' && event.target.value !== '' ? Number(event.target.value) : event.target.value || null)} />}
      {help && <p id={`${id}-help`} className="text-xs text-muted-foreground">{help}</p>}{invalid && <p role="alert" className="text-xs text-destructive">{t('selection.invalidField')}</p>}
    </div>
  })}</div>
}
