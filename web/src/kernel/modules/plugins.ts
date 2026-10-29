import type { WebModule } from './define'

interface WebPluginEntry {
  key: string
  version: string
  entry: string
  style: string | null
}

const PLUGINS_URL = '/api/v1/public/plugins'

let loaded: WebModule[] = []

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
    loaded = results.flatMap((result) => {
      if (result.status === 'fulfilled') return [result.value]
      console.error(result.reason)
      return []
    })
  } catch (error) {
    console.error(error)
  }
}

export function webPlugins(): WebModule[] {
  return loaded
}
