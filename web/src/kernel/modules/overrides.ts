import type { ComponentType } from 'react'
import { useActiveContributions } from './registry'

const warned = new Set<string>()

function warnOverridden(id: string, modules: string[]) {
  if (!import.meta.env.DEV || modules.length < 2) return
  const signature = `${id}:${modules.join(',')}`
  if (warned.has(signature)) return
  warned.add(signature)
  console.warn(`[aphel] "${id}" is overridden by ${modules.join(', ')}; "${modules[modules.length - 1]}" wins`)
}

export function useOverride<P extends object>(id: string, Default: ComponentType<P>): ComponentType<P> {
  const items = useActiveContributions<ComponentType<P>>(`override:${id}`)
  if (items.length === 0) return Default
  warnOverridden(
    id,
    items.map((item) => item.module),
  )
  return items[items.length - 1].value
}
