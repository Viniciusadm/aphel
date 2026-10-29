export type ResolvedTheme = 'light' | 'dark'

export const APP_THEME: ResolvedTheme = 'light'

export function applyResolvedTheme(theme: ResolvedTheme): void {
  document.documentElement.dataset.theme = theme
}
