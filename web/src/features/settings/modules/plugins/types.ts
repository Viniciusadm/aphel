export interface PluginPermission {
  key: string
  label: string
  description: string | null
  grant_to: string[]
}

export interface PluginDeclarations {
  entities: string[]
  extended_types: string[]
  hooks: string[]
  events: string[]
  routes: string[]
  public_routes: string[]
  requires: string[]
  permissions: PluginPermission[]
  allowed_hosts: string[]
  schedules: string[]
  secrets: string[]
  actions: string[]
  emits: string[]
  notifications: string[]
  widgets: string[]
  operations: string[]
  has_web: boolean
}

export interface PluginSetting {
  key: string
  label: string
  type: 'number' | 'boolean' | 'text' | 'secret'
  help: string | null
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
}

export interface PluginConsent extends PluginDeclarations {
  key: string
  name: string
  version: string
  description: string | null
  installed_version: string | null
}

export const PLUGINS_PATH = '/v1/plugins'
export const pluginsKey = ['plugins'] as const
