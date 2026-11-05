import { contrastRatio, MIN_TEXT_CONTRAST } from '@/sdk'
import { cssColorToHex } from './color'
import { TOKENS, resolvedValue, valueToCss } from './tokens'
import type { TokenSets } from './types'

const TEXT_PAIRS: readonly (readonly [string, string])[] = [
  ['foreground', 'background'],
  ['card-foreground', 'card'],
  ['popover-foreground', 'popover'],
  ['primary-foreground', 'primary-solid'],
  ['secondary-foreground', 'secondary'],
  ['muted-foreground', 'background'],
  ['accent-foreground', 'accent'],
  ['destructive-foreground', 'destructive-solid'],
  ['success-foreground', 'success-solid'],
  ['warning-foreground', 'warning-solid'],
]

export interface ContrastWarning {
  foreground: string
  background: string
  ratio: number
}

export function contrastWarnings(sets: TokenSets, mode: 'light' | 'dark'): ContrastWarning[] {
  const lookup = (name: string) => {
    const token = TOKENS.find((candidate) => candidate.name === name)
    return token ? valueToCss(resolvedValue(sets, token, mode)) : undefined
  }
  return TEXT_PAIRS.flatMap(([foreground, background]) => {
    const front = cssColorToHex(lookup(foreground) ?? '', lookup)
    const back = cssColorToHex(lookup(background) ?? '', lookup)
    if (!front || !back) return []
    const ratio = contrastRatio(front, back)
    return ratio < MIN_TEXT_CONTRAST ? [{ foreground, background, ratio }] : []
  })
}
