import { DialogForm } from '@/components/ui/dialog-form'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { api, isApiError } from '@/api'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { fieldViolations } from '@/features/auth/lib/api-errors'
import { NativeSelect } from '@/features/settings/organization/components/NativeSelect'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { SetupStepFrame } from '@/kernel/setup'
import { cn } from '@/lib/utils'
import {
  dependencyClosure,
  needsConfiguration,
  recommendedModules,
  type InstallerCatalog,
  type InstallerModule,
  type ModuleChoice,
  type ModuleSettings,
} from '../lib/modules'
import { Alert } from '@/components/ui/alert'
import { SearchInput } from '@/components/ui/search-input'

interface FeaturesStepProps {
  catalog: InstallerCatalog
  businessType: string | null
  businessLabel: string
  value: ModuleChoice
  onChange: (value: ModuleChoice) => void
  error: string | null
  onNext: () => void
}

function useModuleText(module: InstallerModule) {
  const translate = useModuleTranslation(module.key)
  const name = translate('name')
  const description = translate('description')
  return {
    name: name === 'name' ? module.name : name,
    description: description === 'description' ? module.description : description,
  }
}

function ModuleName({ module }: { module: InstallerModule }) {
  return <>{useModuleText(module).name}</>
}

export function FeaturesStep({ catalog, businessType, businessLabel, value, onChange, error, onNext }: FeaturesStepProps) {
  const { t } = useTranslation('setup')
  const [search, setSearch] = useState('')
  const [editing, setEditing] = useState<string | null>(null)
  const selected = dependencyClosure(catalog.modules, value.modules)
  const recommended = recommendedModules(catalog, businessType)
  const optional = catalog.modules.filter((item) => !item.required)
  const isShelf = (item: InstallerModule) => needsConfiguration(item, catalog.modules)
  const byRecommendation = (left: InstallerModule, right: InstallerModule) =>
    Number(recommended.has(right.key)) - Number(recommended.has(left.key))
  const groups = [
    { key: 'recommended', title: t('features.recommended'), items: optional.filter((item) => !isShelf(item) && recommended.has(item.key)) },
    { key: 'others', title: t('features.others'), items: optional.filter((item) => !isShelf(item) && !recommended.has(item.key)) },
    {
      key: 'shelf',
      title: t('features.needsSetup'),
      hint: t('features.needsSetupHint'),
      items: optional.filter(isShelf).sort(byRecommendation),
    },
  ]
  const count = optional.filter((item) => selected.has(item.key)).length

  const remove = (key: string) => {
    const modules = value.modules.filter((item) => item !== key)
    const remaining = dependencyClosure(catalog.modules, modules)
    onChange({
      modules,
      module_settings: Object.fromEntries(Object.entries(value.module_settings).filter(([item]) => remaining.has(item))),
    })
  }

  const toggle = (item: InstallerModule) => {
    if (selected.has(item.key)) remove(item.key)
    else if (isShelf(item)) setEditing(item.key)
    else onChange({ ...value, modules: [...value.modules, item.key] })
  }

  return (
    <SetupStepFrame
      title={t('features.title')}
      description={businessLabel ? t('features.description', { business: businessLabel }) : t('features.descriptionBlank')}
      error={error}
      primary={{ label: t('actions.next'), onClick: onNext }}
      aside={<span className="text-sm text-muted-foreground sm:px-2">{t('features.count', { count })}</span>}
    >
      <SearchInput
          aria-label={t('features.search')}
          placeholder={t('features.search')}
          value={search}
          onValueChange={(value) => setSearch(value)}
        />
      {groups.map((group) => (
        <FeatureGroup key={group.key} title={group.title} hint={group.hint} search={search} items={group.items}>
          {(item) => {
            const implied = selected.has(item.key) && !value.modules.includes(item.key)
            const locked =
              implied ||
              catalog.modules.some(
                (other) => other.key !== item.key && selected.has(other.key) && other.dependencies.includes(item.key),
              )
            return (
              <FeatureCard
                key={item.key}
                module={item}
                checked={selected.has(item.key)}
                locked={locked}
                recommended={group.key === 'shelf' && recommended.has(item.key)}
                note={implied || locked ? t('features.dependency') : ''}
                configurable={selected.has(item.key) && item.settings_schema.length > 0}
                onToggle={() => toggle(item)}
                onConfigure={() => setEditing(item.key)}
              />
            )
          }}
        </FeatureGroup>
      ))}
      <details className="rounded-lg border px-4 py-3 text-sm">
        <summary className="cursor-pointer font-medium">{t('features.included')}</summary>
        <ul className="mt-2 flex flex-wrap gap-1.5">
          {catalog.modules
            .filter((item) => item.required)
            .map((item) => (
              <li key={item.key}>
                <Badge variant="secondary">
                  <ModuleName module={item} />
                </Badge>
              </li>
            ))}
        </ul>
      </details>
      {editing && (
        <Configuration
          key={editing}
          catalog={catalog}
          moduleKey={editing}
          value={value}
          onCancel={() => setEditing(null)}
          onSave={(next) => {
            onChange(next)
            setEditing(null)
          }}
        />
      )}
    </SetupStepFrame>
  )
}

function FeatureGroup({
  title,
  hint,
  search,
  items,
  children,
}: {
  title: string
  hint?: string
  search: string
  items: InstallerModule[]
  children: (item: InstallerModule) => ReactNode
}) {
  if (items.length === 0) return null
  return (
    <section className="grid gap-3" aria-label={title}>
      <div className="grid gap-0.5">
        <h2 className="text-sm font-semibold">{title}</h2>
        {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
      </div>
      <div className="grid gap-2 sm:grid-cols-2">
        {items.map((item) => (
          <SearchMatch key={item.key} module={item} search={search}>
            {children(item)}
          </SearchMatch>
        ))}
      </div>
    </section>
  )
}

function SearchMatch({ module, search, children }: { module: InstallerModule; search: string; children: ReactNode }) {
  const text = useModuleText(module)
  const query = search.trim().toLocaleLowerCase()
  if (!query) return <>{children}</>
  const haystack = `${module.key} ${module.name} ${text.name} ${text.description}`.toLocaleLowerCase()
  return haystack.includes(query) ? <>{children}</> : null
}

function FeatureCard({
  module,
  checked,
  locked,
  recommended,
  note,
  configurable,
  onToggle,
  onConfigure,
}: {
  module: InstallerModule
  checked: boolean
  locked: boolean
  recommended: boolean
  note: string
  configurable: boolean
  onToggle: () => void
  onConfigure: () => void
}) {
  const { t } = useTranslation('setup')
  const text = useModuleText(module)
  const disabled = locked || !module.compatible
  return (
    <div
      className={cn(
        'grid content-start gap-2 rounded-lg border bg-card p-3 transition-colors',
        checked && 'border-primary bg-primary/5',
        !module.compatible && 'opacity-60',
      )}
    >
      <label className={cn('flex items-start gap-3', !disabled && 'cursor-pointer')}>
        <input
          type="checkbox"
          className="mt-0.5 size-4 shrink-0 accent-primary"
          checked={checked}
          disabled={disabled}
          onChange={onToggle}
        />
        <span className="grid min-w-0 gap-0.5">
          <span className="flex flex-wrap items-center gap-2 text-sm font-medium">
            {text.name}
            {recommended && <Badge variant="secondary">{t('features.recommendedBadge')}</Badge>}
          </span>
          <span className="text-xs text-muted-foreground">{text.description}</span>
          {note && <span className="text-xs text-muted-foreground">{note}</span>}
          {!module.compatible && <span className="text-xs text-destructive">{t('features.incompatible')}</span>}
        </span>
      </label>
      {configurable && (
        <Button type="button" variant="outline" size="sm" className="justify-self-start" onClick={onConfigure}>
          {t('features.editConfiguration')}
        </Button>
      )}
    </div>
  )
}

function Configuration({
  catalog,
  moduleKey,
  value,
  onSave,
  onCancel,
}: {
  catalog: InstallerCatalog
  moduleKey: string
  value: ModuleChoice
  onSave: (value: ModuleChoice) => void
  onCancel: () => void
}) {
  const { t } = useTranslation('setup')
  const closure = dependencyClosure(catalog.modules, [moduleKey])
  const items = catalog.modules.filter((item) => !item.required && closure.has(item.key) && item.settings_schema.length > 0)
  const target = catalog.modules.find((item) => item.key === moduleKey)
  const [settings, setSettings] = useState<ModuleSettings>(() =>
    Object.fromEntries(
      items.map((item) => [
        item.key,
        {
          ...Object.fromEntries(
            item.settings_schema
              .filter((field) => field.type !== 'secret' && field.default != null)
              .map((field) => [field.key, field.default]),
          ),
          ...value.module_settings[item.key],
        },
      ]),
    ),
  )
  const [error, setError] = useState<string | null>(null)
  const [violations, setViolations] = useState<string[]>([])
  const [saving, setSaving] = useState(false)

  const save = async () => {
    setSaving(true)
    setError(null)
    setViolations([])
    const next = {
      modules: dependencyClosure(catalog.modules, value.modules).has(moduleKey) ? value.modules : [...value.modules, moduleKey],
      module_settings: { ...value.module_settings, ...settings },
    }
    try {
      await api.post('/v1/installer/modules/validate', next)
      onSave(next)
    } catch (failure) {
      setError(t('features.invalidConfiguration'))
      if (isApiError(failure)) setViolations(fieldViolations(failure.details).map((item) => item.field))
    } finally {
      setSaving(false)
    }
  }

  return (
    <Dialog open onOpenChange={(open) => !open && !saving && onCancel()}>
      <DialogContent>
        <DialogForm
          noValidate
          onSubmit={(event) => {
            event.preventDefault()
            event.stopPropagation()
            void save()
          }}
          title={<>{target ? <ModuleName module={target} /> : t('features.configureTitle')}</>}
          description={t('features.configurationHint')}
          cancelLabel={t('features.cancel')}
          submitLabel={saving ? t('features.validating') : t('features.add')}
          onCancel={onCancel}
          pending={saving}
          submitDisabled={saving}
        >
          {error && (
            <Alert className="text-destructive">
              {error}
            </Alert>
          )}
          <fieldset disabled={saving} className="grid gap-5">
            {items.map((item) => (
              <ConfigurationFields
                key={item.key}
                module={item}
                showTitle={items.length > 1}
                values={settings[item.key] ?? {}}
                violations={violations}
                onChange={(key, fieldValue) =>
                  setSettings((current) => ({ ...current, [item.key]: { ...current[item.key], [key]: fieldValue } }))
                }
              />
            ))}
          </fieldset>
        </DialogForm>
      </DialogContent>
    </Dialog>
  )
}

function ConfigurationFields({
  module,
  showTitle,
  values,
  violations,
  onChange,
}: {
  module: InstallerModule
  showTitle: boolean
  values: Record<string, unknown>
  violations: string[]
  onChange: (key: string, value: unknown) => void
}) {
  const translate = useModuleTranslation(module.key)
  const { t } = useTranslation('setup')
  const lookup = (key: string, fallback: string) => {
    const result = translate(key)
    return result === key ? fallback : result
  }
  return (
    <div className="grid gap-3">
      {showTitle && (
        <h3 className="font-semibold">
          <ModuleName module={module} />
        </h3>
      )}
      {module.settings_schema.map((field) => {
        const id = `setup-setting-${module.key}-${field.key}`
        const invalid = violations.some(
          (path) => path === `module_settings.${module.key}.settings.${field.key}` || path === `module_settings.${module.key}.${field.key}`,
        )
        const help = lookup(`settings.${field.key}.help`, field.help ?? '')
        const inputType =
          field.type === 'secret'
            ? 'password'
            : field.type === 'number'
              ? 'number'
              : field.type === 'email'
                ? 'email'
                : field.type === 'url'
                  ? 'url'
                  : 'text'
        return (
          <div key={field.key} className="grid gap-1.5">
            <label htmlFor={id} className="text-sm font-medium">
              {lookup(`settings.${field.key}.label`, field.label)}
              {field.required ? ' *' : ''}
            </label>
            {field.type === 'boolean' ? (
              <input
                id={id}
                type="checkbox"
                className="size-4 accent-primary"
                checked={values[field.key] === true}
                onChange={(event) => onChange(field.key, event.target.checked)}
              />
            ) : field.type === 'select' ? (
              <NativeSelect
                id={id}
                required={field.required}
                aria-invalid={invalid || undefined}
                value={String(values[field.key] ?? '')}
                onChange={(event) => onChange(field.key, event.target.value)}
              >
                <option value="">{t('features.selectOption')}</option>
                {field.options.map((option) => (
                  <option key={option.value} value={option.value}>
                    {lookup(`settings.${field.key}.options.${option.value}`, option.label)}
                  </option>
                ))}
              </NativeSelect>
            ) : (
              <Input placeholder={t('placeholders.value')}
                id={id}
                type={inputType}
                required={field.required}
                autoComplete={field.type === 'secret' ? 'new-password' : 'off'}
                min={field.min ?? undefined}
                max={field.max ?? undefined}
                step="any"
                maxLength={field.type === 'secret' ? 4096 : field.type === 'long_text' ? 20000 : 500}
                aria-invalid={invalid || undefined}
                aria-describedby={help ? `${id}-help` : undefined}
                value={String(values[field.key] ?? '')}
                onChange={(event) =>
                  onChange(
                    field.key,
                    field.type === 'number' && event.target.value !== '' ? Number(event.target.value) : event.target.value || null,
                  )
                }
              />
            )}
            {help && (
              <p id={`${id}-help`} className="text-xs text-muted-foreground">
                {help}
              </p>
            )}
            {invalid && (
              <SharedFieldError>
                {t('features.invalidField')}
              </SharedFieldError>
            )}
          </div>
        )
      })}
    </div>
  )
}
