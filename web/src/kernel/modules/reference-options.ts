import type { ReferenceOptionsContext, ReferenceOptionsProvider, ReferenceOptionsResult } from './define'
import {
  activeContributions,
  contributions,
  isModuleActive,
  sortContributions,
  useActiveContributions,
  useModuleState,
} from './registry'

const EMPTY_RESULT: ReferenceOptionsResult = { options: [] }
const NO_CONTEXT: ReferenceOptionsContext = {}

export function useReferenceOptions(kind: string, context: ReferenceOptionsContext = NO_CONTEXT): ReferenceOptionsResult {
  const current = useModuleState()
  let result = EMPTY_RESULT
  for (const { module, value } of sortContributions(contributions<ReferenceOptionsProvider>(`reference:${kind}`))) {
    const active = isModuleActive(module, current)
    const options = value.use({ ...context, enabled: (context.enabled ?? true) && active })
    if (active) result = options
  }
  return result
}

export function useHasReferenceOptions(kind: string): boolean {
  return useActiveContributions<ReferenceOptionsProvider>(`reference:${kind}`).length > 0
}

export function useReferenceQueryKeys(kinds: string[]): (readonly unknown[])[] {
  const current = useModuleState()
  return kinds.flatMap((kind) => {
    const queryKey = activeContributions<ReferenceOptionsProvider>(`reference:${kind}`, current).at(-1)?.value.queryKey
    return queryKey ? [queryKey] : []
  })
}
