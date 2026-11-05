import { useTranslation } from 'react-i18next'
import type { PluginDeclarations } from './types'

const DECLARATIONS = [
  ['requires', (plugin: PluginDeclarations) => plugin.requires],
  ['optionalRequires', (plugin: PluginDeclarations) => plugin.optional_requires],
  ['replaces', (plugin: PluginDeclarations) => plugin.replaces],
  ['permissions', (plugin: PluginDeclarations) => plugin.permissions.map((permission) => permission.label)],
  ['allowedHosts', (plugin: PluginDeclarations) => plugin.allowed_hosts],
  ['publicRoutes', (plugin: PluginDeclarations) => plugin.public_routes],
  ['schedules', (plugin: PluginDeclarations) => plugin.schedules],
  ['secrets', (plugin: PluginDeclarations) => plugin.secrets],
  ['operations', (plugin: PluginDeclarations) => plugin.operations],
  ['actions', (plugin: PluginDeclarations) => plugin.actions],
  ['emits', (plugin: PluginDeclarations) => plugin.emits],
  ['notifications', (plugin: PluginDeclarations) => plugin.notifications],
  ['widgets', (plugin: PluginDeclarations) => plugin.widgets],
  ['rolePresets', (plugin: PluginDeclarations) => plugin.role_presets.map((preset) => preset.label)],
  [
    'entityScopes',
    (plugin: PluginDeclarations) =>
      plugin.entity_scopes.map((entity) => `${entity.record_type} (${entity.permission_scope})`),
  ],
  ['uniqueFields', (plugin: PluginDeclarations) => plugin.unique_fields],
  ['entities', (plugin: PluginDeclarations) => plugin.entities],
  ['extendedTypes', (plugin: PluginDeclarations) => plugin.extended_types],
  ['hooks', (plugin: PluginDeclarations) => plugin.hooks],
  ['events', (plugin: PluginDeclarations) => plugin.events],
  ['routes', (plugin: PluginDeclarations) => plugin.routes],
] as const

export function PluginDeclarationList({ plugin, compact = false }: { plugin: PluginDeclarations; compact?: boolean }) {
  const { t } = useTranslation('modules')
  const rows = DECLARATIONS.map(([label, values]) => [label, values(plugin)] as const).filter(
    ([, values]) => values.length > 0,
  )
  if (compact) {
    return (
      <section className="flex min-h-0 flex-1 flex-col gap-2 overflow-hidden rounded-md bg-muted/40 p-3">
        <h3 className="shrink-0 text-xs font-semibold text-foreground">{t('plugins.declarationsPreview')}</h3>
        {rows.length === 0 ? (
          <p className="text-xs text-muted-foreground">{t('plugins.noDeclarations')}</p>
        ) : (
          <dl className="grid content-start gap-2 overflow-hidden">
            {rows.slice(0, 3).map(([label, values]) => (
              <div key={label} className="grid min-w-0 gap-1">
                <dt className="text-[11px] font-medium leading-tight text-muted-foreground">
                  {t(`plugins.declares.${label}`)}
                </dt>
                <dd className="flex min-w-0 flex-wrap gap-1 overflow-hidden">
                  {values.slice(0, 3).map((value) => (
                    <span key={value} className="max-w-full truncate rounded bg-background px-1.5 py-0.5 text-xs text-foreground">
                      {value}
                    </span>
                  ))}
                  {values.length > 3 && (
                    <span className="rounded bg-background px-1.5 py-0.5 text-xs text-muted-foreground">
                      +{values.length - 3}
                    </span>
                  )}
                </dd>
              </div>
            ))}
          </dl>
        )}
      </section>
    )
  }
  if (rows.length === 0) return null
  return (
    <dl className="grid gap-1 text-xs">
      {rows.map(([label, values]) => (
        <div key={label} className="flex flex-wrap gap-1">
          <dt className="font-medium">{t(`plugins.declares.${label}`)}:</dt>
          <dd className="break-words text-muted-foreground">{values.join(', ')}</dd>
        </div>
      ))}
    </dl>
  )
}
