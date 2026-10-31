import { resolvedValue, TOKENS, valueToCss } from './lib/tokens'
import type { TokenSets } from './lib/types'

const SWATCH_TOKENS = ['background', 'card', 'primary-solid', 'secondary', 'accent', 'destructive-solid'] as const

export function ThemeSwatches({ tokens }: { tokens: TokenSets }) {
  return (
    <div className="flex overflow-hidden rounded-md border" aria-hidden="true">
      {SWATCH_TOKENS.map((name) => {
        const token = TOKENS.find((candidate) => candidate.name === name)
        return (
          <span
            key={name}
            className="h-8 flex-1"
            style={{ backgroundColor: token ? valueToCss(resolvedValue(tokens, token, 'light')) : undefined }}
          />
        )
      })}
    </div>
  )
}
