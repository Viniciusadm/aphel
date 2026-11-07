export interface PluginPermission {
  key: string
  label: string
  description: string | null
  grant_to: string[]
}

export interface PluginRolePreset {
  key: string
  label: string
  description: string | null
  permissions: string[]
  scope: 'own' | 'team' | 'organization'
}

export interface PluginEntityScope {
  record_type: string
  permission_scope: 'own' | 'team' | 'organization'
}

export interface RoleGrantStatus {
  role: string
  label: string | null
  preset: boolean
  role_exists: boolean
  expected: number
  granted: number
  missing: string[]
}

export interface PluginDeclarations {
  entities: string[]
  extended_types: string[]
  hooks: string[]
  events: string[]
  routes: string[]
  public_routes: string[]
  requires: string[]
  optional_requires: string[]
  permissions: PluginPermission[]
  allowed_hosts: string[]
  schedules: string[]
  secrets: string[]
  actions: string[]
  emits: string[]
  ui: string | null
  notifications: string[]
  widgets: string[]
  operations: string[]
  role_presets: PluginRolePreset[]
  entity_scopes: PluginEntityScope[]
  unique_fields: string[]
  menu: PluginMenu
  replaces: string[]
  has_web: boolean
}

export interface PluginMenu {
  flat?: boolean
  own_entities?: 'visible' | 'hidden' | null
}

export interface PluginConflict {
  kind: 'hook_overlap' | 'role_preset_key'
  target: string
  other_plugin: string
  other_name: string
}

export interface ModuleConflictPlan {
  module: string
  name: string
  enabled: boolean
  blocked_by: string[]
}

export interface PluginSetting {
  key: string
  label: string
  type: 'number' | 'boolean' | 'text' | 'select' | 'secret' | 'tls_identity'
  help: string | null
  group?: string | null
  options?: { value: string; label: string }[]
  min: number | null
  max: number | null
  value: unknown
  configured: boolean | null
}

export interface InstalledPlugin extends PluginDeclarations {
  key: string
  name: string
  version: string
  description: string | null
  enabled: boolean
  backend_error: string | null
  settings: PluginSetting[]
  grants: RoleGrantStatus[]
  ungranted_roles: string[]
}

export interface PluginConsent extends PluginDeclarations {
  key: string
  name: string
  version: string
  description: string | null
  installed_version: string | null
  grants: RoleGrantStatus[]
  conflicts: PluginConflict[]
  module_conflicts: ModuleConflictPlan[]
}

export const PLUGINS_PATH = '/v1/wasm-runtime'
export const pluginsKey = ['plugins'] as const
