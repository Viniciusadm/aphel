import { createContext, useContext } from 'react'
import type { ResolvedTheme } from '@/lib/theme'

export interface ThemeContextValue {
  resolvedTheme: ResolvedTheme
}

export const ThemeContext = createContext<ThemeContextValue | null>(null)

export function useTheme(): ThemeContextValue {
  const context = useContext(ThemeContext)
  if (!context) throw new Error('useTheme must be used inside ThemeProvider')
  return context
}
