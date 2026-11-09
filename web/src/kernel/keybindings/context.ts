import { createContext, useContext } from 'react'
import type { KeybindingAction, KeybindingHandler, KeybindingOverrides, KeybindingScope } from './types'

export interface KeybindingContextValue {
  actions: KeybindingAction[]
  scopes: KeybindingScope[]
  activeScopes: string[]
  enabled: boolean
  organization: KeybindingOverrides
  user: KeybindingOverrides
  registerHandler: (id: string, handler: { current: KeybindingHandler }) => () => void
  activateScope: (scope: string) => () => void
}

const noop = () => () => {}

const FALLBACK: KeybindingContextValue = {
  actions: [],
  scopes: [],
  activeScopes: [],
  enabled: false,
  organization: {},
  user: {},
  registerHandler: noop,
  activateScope: noop,
}

export const KeybindingContext = createContext<KeybindingContextValue>(FALLBACK)

export function useKeybindings(): KeybindingContextValue {
  return useContext(KeybindingContext)
}
