import type { RouteObject } from 'react-router'

export const SITE_SCREENS = [
  { key: 'frame', path: '/site/header-footer', component: 'SiteFramePage' },
  { key: 'appearance', path: '/site/appearance', component: 'SiteAppearancePage' },
  { key: 'languages', path: '/site/languages', component: 'SiteLanguagesPage' },
  { key: 'customers', path: '/site/customers', component: 'SiteCustomersPage' },
  { key: 'redirects', path: '/site/redirects', component: 'SiteRedirectsPage' },
  { key: 'general', path: '/site/general', component: 'SiteGeneralPage' },
] as const

export const siteRoutes: RouteObject[] = SITE_SCREENS.map((screen) => ({
  path: screen.path.slice(1),
  lazy: async () => ({ Component: (await import('./SiteScreens'))[screen.component] }),
}))
