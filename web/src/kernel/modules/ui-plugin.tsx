import { useEffect, type ReactNode } from 'react'
import { usePreferences } from '@/features/command-palette/preferences/api'
import { useModuleState } from './registry'
import { UiPluginContext, stylesheetFor, type UiPluginState } from './ui-plugin-context'
import { loadedUiPlugin, readUiChoice, writeUiChoice } from './wasm_runtime'

const RELOAD_GUARD = 'aphel.ui.reloaded'

function applyStylesheet(choice: string | null) {
  const link = document.getElementById('aphel-theme')
  if (link instanceof HTMLLinkElement) link.href = stylesheetFor(choice)
}

function guardReload(target: string | null): boolean {
  try {
    if (window.sessionStorage.getItem(RELOAD_GUARD) === (target ?? '')) return false
    window.sessionStorage.setItem(RELOAD_GUARD, target ?? '')
    return true
  } catch {
    return false
  }
}

export function UiPluginProvider({ children }: { children: ReactNode }) {
  const preferences = usePreferences()
  const modules = useModuleState()
  const settled = !preferences.isPending || preferences.fetchStatus === 'paused'
  const data = preferences.isError ? undefined : preferences.data
  const effective = data ? data.effective_ui_plugin ?? null : loadedUiPlugin()

  useEffect(() => {
    if (!data) return
    const stored = data.ui_plugin ?? null
    if (stored !== readUiChoice()) writeUiChoice(stored)
    applyStylesheet(stored)
    const loaded = loadedUiPlugin()
    if ((data.effective_ui_plugin ?? null) !== loaded && guardReload(data.effective_ui_plugin ?? null)) {
      window.location.reload()
    }
  }, [data])

  const state: UiPluginState = { plugin: effective, ready: settled && modules.loaded }
  return <UiPluginContext.Provider value={state}>{children}</UiPluginContext.Provider>
}
