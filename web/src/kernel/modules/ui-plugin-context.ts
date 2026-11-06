import { createContext, useContext, type ComponentType } from 'react'
import { contractBucket } from './contract-components'
import { activeContributions, useModuleState } from './registry'

export const CORE_UI = 'core'

export interface UiPluginState {
  plugin: string | null
  ready: boolean
}

export const UiPluginContext = createContext<UiPluginState>({ plugin: null, ready: true })

export function stylesheetFor(choice: string | null): string {
  return `/api/v1/public/ui/${choice ?? 'default'}.css`
}

export function useUiPlugin(): UiPluginState {
  return useContext(UiPluginContext)
}

export function useSurface<P extends object, X extends object = object>(
  contract: string,
): { plugin: string | null; Component: (ComponentType<P> & X) | undefined } {
  const { plugin } = useUiPlugin()
  const modules = useModuleState()
  if (!plugin) return { plugin: null, Component: undefined }
  const matches = activeContributions<ComponentType<P> & X>(contractBucket(contract), modules).filter(({ module }) => module === plugin)
  return { plugin, Component: matches.length === 1 ? matches[0].value : undefined }
}
