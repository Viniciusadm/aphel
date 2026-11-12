import { Navigate, type RouteObject } from 'react-router'

export const SITE_SCREENS = [
  { key: 'navigation', path: '/site/navigation', component: 'SiteNavigationPage' },
  { key: 'frame', path: '/site/header-footer', component: 'SiteFramePage' },
  { key: 'appearance', path: '/site/appearance', component: 'SiteAppearancePage' },
  { key: 'languages', path: '/site/languages', component: 'SiteLanguagesPage' },
  { key: 'customers', path: '/site/customers', component: 'SiteCustomersPage' },
  { key: 'redirects', path: '/site/redirects', component: 'SiteRedirectsPage' },
  { key: 'general', path: '/site/general', component: 'SiteGeneralPage' },
] as const

export const SITE_OVERVIEW_PATH = '/site/overview'

export const siteRoutes: RouteObject[] = [
  { path: 'site', element: <Navigate to={SITE_OVERVIEW_PATH} replace /> },
  { path: SITE_OVERVIEW_PATH.slice(1), lazy: async () => ({ Component: (await import('./SiteOverviewPage')).SiteOverviewPage }) },
  ...SITE_SCREENS.map((screen) => ({
    path: screen.path.slice(1),
    lazy: async () => ({ Component: (await import('./SiteScreens'))[screen.component] }),
  })),
]
