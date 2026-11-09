import type { CommandHelpers, KeybindingScopeKind } from '@/kernel/modules/define'

export type { KeybindingScopeKind }

export const GLOBAL_SCOPE = 'global'
export const SEQUENCE_TIMEOUT_MS = 1200

export type KeybindingOverrides = Record<string, string[]>

export type KeybindingSource = 'default' | 'organization' | 'user'

export type KeybindingLayer = 'organization' | 'user'

export interface KeybindingScope {
  id: string
  module: string
  label: string
  description?: string
  kind: KeybindingScopeKind | 'global'
}

export interface KeybindingAction {
  id: string
  module: string
  label: string
  description?: string
  scope: string
  defaults: string[]
  keys: string[]
  source: KeybindingSource
  allowInEditable: boolean
  always: boolean
  repeat: boolean
  to?: string
  run?: (helpers: CommandHelpers) => void
}

export type KeybindingHandler = (event: KeyboardEvent) => boolean | void
