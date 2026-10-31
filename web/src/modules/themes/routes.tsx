import type { RouteObject } from 'react-router'

export const themesSettingsRoute: RouteObject = {
  path: 'themes',
  children: [
    { index: true, lazy: async () => ({ Component: (await import('./ThemesPage')).ThemesPage }) },
    { path: ':key', lazy: async () => ({ Component: (await import('./ThemeEditorPage')).ThemeEditorPage }) },
  ],
}
