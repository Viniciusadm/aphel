import type { WebModule } from './define'

export interface PluginMenuRule {
  target: string
  action: 'hide' | 'move'
  order?: number | null
}

export interface PluginMenu {
  flat: boolean
  own_entities?: 'visible' | 'hidden' | null
  rules: PluginMenuRule[]
}

interface WebPluginEntry {
  key: string
  name: string
  menu: PluginMenu
  role_homes?: Record<string, string>
  version: string
  entry: string
  style: string | null
}

const PLUGINS_URL = '/api/v1/public/wasm-runtime'

let loaded: WebModule[] = []
let failedNames: string[] = []
let menus: ReadonlyMap<string, PluginMenu> = new Map()
let roleHomes: ReadonlyMap<string, string> = new Map()
let names: ReadonlyMap<string, string> = new Map()

function injectStyle(href: string) {
  const link = document.createElement('link')
  link.rel = 'stylesheet'
  link.href = href
  document.head.append(link)
}

async function importPlugin(plugin: WebPluginEntry): Promise<WebModule> {
  if (plugin.style) injectStyle(plugin.style)
  const imported = (await import(/* @vite-ignore */ plugin.entry)) as { default?: WebModule }
  if (!imported.default || imported.default.key !== plugin.key) {
    throw new Error(`plugin ${plugin.key} does not export its module as default`)
  }
  return imported.default
}

export async function loadWebPlugins(): Promise<void> {
  try {
    const response = await fetch(PLUGINS_URL, { headers: { Accept: 'application/json' } })
    if (!response.ok) return
    const body = (await response.json()) as { data: WebPluginEntry[] }
    const results = await Promise.allSettled(body.data.map(importPlugin))
    const loadedMenus = new Map<string, PluginMenu>()
    const loadedHomes = new Map<string, string>()
    const loadedNames = new Map<string, string>()
    const failures: string[] = []
    loaded = results.flatMap((result, index) => {
      if (result.status === 'fulfilled') {
        loadedMenus.set(body.data[index].key, body.data[index].menu)
        loadedNames.set(body.data[index].key, body.data[index].name)
        for (const [role, path] of Object.entries(body.data[index].role_homes ?? {})) loadedHomes.set(role, path)
        return [result.value]
      }
      console.error(result.reason)
      failures.push(body.data[index].name)
      return []
    })
    failedNames = failures
    menus = loadedMenus
    roleHomes = loadedHomes
    names = loadedNames
  } catch (error) {
    console.error(error)
  }
}

export function webPlugins(): WebModule[] {
  return loaded
}

export function pluginMenus(): ReadonlyMap<string, PluginMenu> {
  return menus
}

export function failedWebPlugins(): string[] {
  return failedNames
}

export function pluginRoleHome(roleKey: string): string | undefined {
  return roleHomes.get(roleKey)
}

export function pluginDisplayName(key: string): string {
  return names.get(key) ?? key
}
