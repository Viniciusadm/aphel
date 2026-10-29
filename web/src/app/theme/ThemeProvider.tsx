import { useEffect, type ReactNode } from 'react'
import { APP_THEME, applyResolvedTheme } from '@/lib/theme'
import { ThemeContext, type ThemeContextValue } from './theme-context'

const value: ThemeContextValue = { resolvedTheme: APP_THEME }

export function ThemeProvider({ children }: { children: ReactNode }) {
  useEffect(() => {
    applyResolvedTheme(APP_THEME)
  }, [])

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
}
