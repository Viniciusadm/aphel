import { useCallback, useSyncExternalStore } from 'react'
import { useTranslation } from 'react-i18next'
import i18n, { DEFAULT_LANGUAGE } from '@/i18n'
import type { ModuleLocale, ModuleTranslations, TranslationLoader, TranslationResources, WebModule } from './define'
import { isModuleActive, moduleState, registeredModules, subscribeModules, type ModuleState } from './registry'

export type ModuleTranslate = (key: string, options?: Record<string, unknown>) => string

const SUPPORTED_LOCALES: readonly ModuleLocale[] = ['pt-BR', 'en']

const installed = new Map<string, string[]>()
const owners = new Map<string, Set<string>>()
const applied = new Set<string>()
const resolving = new Map<TranslationLoader, Promise<TranslationResources>>()
const readyListeners = new Set<() => void>()
let pending = 0
let ready = false
let booted = false
let languageRequest = 0

function moduleNamespaces(module: WebModule): [string, ModuleTranslations][] {
  const own: [string, ModuleTranslations][] = module.translations ? [[module.key, module.translations]] : []
  return [...own, ...Object.entries(module.namespaces ?? {})]
}

function isLocale(value: string): value is ModuleLocale {
  return (SUPPORTED_LOCALES as readonly string[]).includes(value)
}

function localesFor(language: string = i18n.language): ModuleLocale[] {
  return [...new Set([language, DEFAULT_LANGUAGE])].filter(isLocale)
}

function unwrap(loaded: TranslationResources | { default: TranslationResources }): TranslationResources {
  return 'default' in loaded && typeof loaded.default === 'object' && loaded.default !== null
    ? (loaded.default as TranslationResources)
    : loaded
}

function resolveLoader(loader: TranslationLoader): Promise<TranslationResources> {
  let promise = resolving.get(loader)
  if (!promise) {
    promise = loader().then(unwrap)
    promise.catch(() => resolving.delete(loader))
    resolving.set(loader, promise)
  }
  return promise
}

function setReady() {
  if (ready || !booted) return
  ready = true
  for (const listener of readyListeners) listener()
}

function track<T>(promise: Promise<T>): Promise<T> {
  pending += 1
  const settle = () => {
    pending -= 1
    if (pending === 0) setReady()
  }
  promise.then(settle, settle)
  return promise
}

function bundleKey(module: string, namespace: string, locale: string) {
  return `${module}\u0000${namespace}\u0000${locale}`
}

function install(module: WebModule) {
  if (installed.has(module.key)) return
  const namespaces = moduleNamespaces(module).map(([namespace]) => namespace)
  for (const namespace of namespaces) owners.set(namespace, (owners.get(namespace) ?? new Set()).add(module.key))
  installed.set(module.key, namespaces)
}

function load(module: WebModule, locales: ModuleLocale[]): Promise<void> {
  install(module)
  const loads: Promise<void>[] = []
  for (const [namespace, translations] of moduleNamespaces(module)) {
    for (const locale of locales) {
      const key = bundleKey(module.key, namespace, locale)
      const source = translations[locale]
      if (!source || applied.has(key)) continue
      if (typeof source !== 'function') {
        i18n.addResourceBundle(locale, namespace, source, true, true)
        applied.add(key)
        continue
      }
      loads.push(
        resolveLoader(source).then((resources) => {
          if (!installed.has(module.key) || applied.has(key)) return
          i18n.addResourceBundle(locale, namespace, resources, true, true)
          applied.add(key)
        }),
      )
    }
  }
  return loads.length > 0 ? track(Promise.all(loads).then(() => undefined)) : Promise.resolve()
}

function unload(module: WebModule) {
  const namespaces = installed.get(module.key)
  if (!namespaces) return
  installed.delete(module.key)
  for (const namespace of namespaces) {
    for (const locale of SUPPORTED_LOCALES) applied.delete(bundleKey(module.key, namespace, locale))
    const remaining = owners.get(namespace)
    remaining?.delete(module.key)
    if (remaining && remaining.size > 0) continue
    owners.delete(namespace)
    for (const locale of Object.keys(i18n.services.resourceStore.data)) i18n.removeResourceBundle(locale, namespace)
  }
}

function activeModules(state: ModuleState): WebModule[] {
  return registeredModules().filter((module) => isModuleActive(module.key, state))
}

export function syncModuleTranslations(): Promise<void> {
  const state = moduleState()
  const locales = localesFor()
  const loads: Promise<void>[] = []
  for (const module of registeredModules()) {
    if (isModuleActive(module.key, state)) loads.push(load(module, locales))
    else unload(module)
  }
  return Promise.all(loads).then(() => undefined)
}

export function preloadInstallerTranslations(): Promise<void> {
  return Promise.all(registeredModules().map((module) => load(module, localesFor()))).then(() => undefined)
}

export function preloadModuleTranslations(state: ModuleState): Promise<void> {
  const locales = localesFor()
  return Promise.all(activeModules(state).map((module) => load(module, locales))).then(() => undefined)
}

export function switchLanguage(language: string): Promise<void> {
  const request = ++languageRequest
  const loads = activeModules(moduleState()).map((module) => load(module, localesFor(language)))
  return track(
    Promise.allSettled(loads).then(async () => {
      if (request === languageRequest && i18n.language !== language) await i18n.changeLanguage(language)
    }),
  )
}

export function installModuleTranslations(): () => void {
  void syncModuleTranslations()
  booted = true
  if (pending === 0) setReady()
  const sync = () => void syncModuleTranslations()
  i18n.on('languageChanged', sync)
  const unsubscribe = subscribeModules(sync)
  return () => {
    i18n.off('languageChanged', sync)
    unsubscribe()
  }
}

function subscribeReady(listener: () => void) {
  readyListeners.add(listener)
  return () => readyListeners.delete(listener)
}

function isReady() {
  return ready
}

export function useModuleTranslationsReady(): boolean {
  return useSyncExternalStore(subscribeReady, isReady, isReady)
}

export function useModuleTranslation(moduleKey: string): ModuleTranslate {
  const { t } = useTranslation()
  return useCallback(
    (key: string, options?: Record<string, unknown>) =>
      String((t as unknown as (key: string, options: Record<string, unknown>) => string)(key, { ...options, ns: moduleKey })),
    [t, moduleKey],
  )
}

export function useLabelTranslator(): (moduleKey: string, label: string) => string {
  const { t } = useTranslation()
  return useCallback(
    (moduleKey: string, label: string) =>
      String((t as unknown as (key: string, options: Record<string, unknown>) => string)(label, { ns: moduleKey })),
    [t],
  )
}
