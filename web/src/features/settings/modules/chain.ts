import type { ModuleInfo } from '@/kernel/modules/api'

export const moduleAnchor = (key: string) => `module-${key}`

export function enableChain(modules: ModuleInfo[], key: string): ModuleInfo[] {
  const byKey = new Map(modules.map((module) => [module.key, module]))
  const ordered: ModuleInfo[] = []
  const seen = new Set<string>()
  const visit = (current: string) => {
    if (seen.has(current)) return
    seen.add(current)
    const module = byKey.get(current)
    if (!module) return
    for (const dependency of module.dependencies) visit(dependency.key)
    if (!module.enabled && !module.required) ordered.push(module)
  }
  visit(key)
  return ordered
}

export function disableChain(modules: ModuleInfo[], key: string): ModuleInfo[] {
  const byKey = new Map(modules.map((module) => [module.key, module]))
  const ordered: ModuleInfo[] = []
  const seen = new Set<string>()
  const visit = (current: string) => {
    if (seen.has(current)) return
    seen.add(current)
    const module = byKey.get(current)
    if (!module) return
    for (const dependent of module.enabled_dependents) visit(dependent)
    if (module.enabled && !module.required) ordered.push(module)
  }
  visit(key)
  return ordered
}
