import { useTranslation } from 'react-i18next'
import type { PluginDeclarations } from './types'

const DECLARATIONS = [
  ['requires', (plugin: PluginDeclarations) => plugin.requires],
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

export function PluginDeclarationList({ plugin }: { plugin: PluginDeclarations }) {
  const { t } = useTranslation('modules')
  const rows = DECLARATIONS.map(([label, values]) => [label, values(plugin)] as const).filter(
    ([, values]) => values.length > 0,
  )
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
