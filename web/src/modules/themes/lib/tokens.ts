import { TOKENS, type TokenDefinition, type TokenGroup } from '@/sdk/tokens.generated'
import type { FontStack, TokenMap, TokenScope, TokenSets, TokenValue } from './types'

export { TOKENS }
export type { TokenDefinition, TokenGroup }

export const TOKEN_GROUP_ORDER: readonly TokenGroup[] = ['color', 'shape', 'size', 'depth', 'type', 'motion']

export function scopeOf(token: TokenDefinition): 'color' | 'shared' {
  return token.kind === 'color' ? 'color' : 'shared'
}

export function defaultValue(token: TokenDefinition, mode: 'light' | 'dark'): string {
  return mode === 'dark' ? (token.dark ?? token.light) : token.light
}

export function stackToCss(stack: FontStack): string {
  return [...stack.families.map((family) => `'${family}'`), stack.fallback].join(', ')
}

export function valueToCss(value: TokenValue): string {
  return typeof value === 'string' ? value : stackToCss(value)
}

export function resolvedValue(sets: TokenSets, token: TokenDefinition, mode: 'light' | 'dark'): TokenValue {
  const scoped: TokenMap = sets[mode]
  return scoped[token.name] ?? sets.shared[token.name] ?? defaultValue(token, mode)
}

export function previewStyle(sets: TokenSets, mode: 'light' | 'dark'): Record<string, string> {
  return Object.fromEntries(
    TOKENS.map((token) => [`--${token.name}`, valueToCss(resolvedValue(sets, token, mode))]),
  )
}

export function emptyTokenSets(): TokenSets {
  return { shared: {}, light: {}, dark: {} }
}

export function setTokenValue(
  sets: TokenSets,
  scope: TokenScope,
  name: string,
  value: TokenValue | undefined,
): TokenSets {
  const next: TokenMap = { ...sets[scope] }
  if (value === undefined) delete next[name]
  else next[name] = value
  return { ...sets, [scope]: next }
}

export function countTokens(sets: TokenSets): number {
  return Object.keys(sets.shared).length + Object.keys(sets.light).length + Object.keys(sets.dark).length
}
