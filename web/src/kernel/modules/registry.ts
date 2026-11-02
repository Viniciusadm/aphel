import { useContext, useMemo, useSyncExternalStore } from 'react'
import { SessionContext } from '@/app/session/session-context'
import type { ModuleApp, PermissionRequirement, WebModule } from './define'

export interface Contribution<T> {
  module: string
  value: T
  order: number
  seq: number
}

export interface ServerModule {
  key: string
  enabled: boolean
}

export interface ModuleState {
  loaded: boolean
  enabled: ReadonlySet<string>
  known: ReadonlySet<string>
}

const DEFAULT_ORDER = 1000

const modules = new Map<string, WebModule>()
const buckets = new Map<string, Contribution<unknown>[]>()
const listeners = new Set<() => void>()
let sequence = 0
let state: ModuleState = { loaded: false, enabled: new Set(), known: new Set() }

export function contribute<T>(bucket: string, module: string, value: T, order = DEFAULT_ORDER) {
  const list = buckets.get(bucket) ?? []
  list.push({ module, value, order, seq: sequence++ })
  buckets.set(bucket, list)
}

export function contributions<T>(bucket: string): Contribution<T>[] {
  return (buckets.get(bucket) ?? []) as Contribution<T>[]
}

export function sortContributions<T>(list: Contribution<T>[]): Contribution<T>[] {
  return [...list].sort((left, right) => left.order - right.order || left.seq - right.seq)
}

export function registerModules(list: WebModule[], createApp: (key: string) => ModuleApp) {
  for (const module of list) {
    if (modules.has(module.key)) throw new Error(`web module "${module.key}" registered twice`)
    modules.set(module.key, module)
  }
  for (const module of list) module.register?.(createApp(module.key))
}

export function registeredModules(): WebModule[] {
  return [...modules.values()]
}

export function moduleDefinition(key: string): WebModule | undefined {
  return modules.get(key)
}

export function isModuleActive(key: string, current: ModuleState = state): boolean {
  if (key === 'kernel') return true
  if (!current.loaded) return false
  return current.known.has(key) && current.enabled.has(key)
}

export function isModuleRequired(key: string): boolean {
  return modules.get(key)?.required === true
}

export function activeContributions<T>(bucket: string, current: ModuleState = state): Contribution<T>[] {
  return sortContributions(contributions<T>(bucket).filter((item) => isModuleActive(item.module, current)))
}

function sameSet(left: ReadonlySet<string>, right: ReadonlySet<string>) {
  return left.size === right.size && [...left].every((key) => right.has(key))
}

export function serverModuleState(list: ServerModule[] | null): ModuleState {
  const known = new Set(list ? list.map((module) => module.key) : modules.keys())
  const enabled = new Set((list ?? []).filter((module) => module.enabled).map((module) => module.key))
  return { loaded: true, enabled, known }
}

export function setServerModules(list: ServerModule[] | null) {
  const next = serverModuleState(list)
  if (state.loaded && sameSet(state.known, next.known) && sameSet(state.enabled, next.enabled)) return
  state = next
  for (const listener of listeners) listener()
}

export function moduleState(): ModuleState {
  return state
}

export function subscribeModules(listener: () => void): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

export function useModuleState(): ModuleState {
  return useSyncExternalStore(subscribeModules, moduleState)
}

export function useIsModuleActive(key: string): boolean {
  return isModuleActive(key, useModuleState())
}

export function useActiveContributions<T>(bucket: string): Contribution<T>[] {
  const current = useModuleState()
  return useMemo(() => activeContributions<T>(bucket, current), [bucket, current])
}

export function permits(requirement: PermissionRequirement | undefined, hasPermission: (permission: string) => boolean) {
  if (!requirement) return true
  const permissions = Array.isArray(requirement) ? requirement : [requirement]
  return permissions.length === 0 || permissions.some((permission) => hasPermission(permission))
}

const denyAll = () => false

export function usePermissionCheck(): (requirement: PermissionRequirement | undefined) => boolean {
  const session = useContext(SessionContext)
  const hasPermission = session?.hasPermission ?? denyAll
  return useMemo(() => (requirement) => permits(requirement, hasPermission), [hasPermission])
}
