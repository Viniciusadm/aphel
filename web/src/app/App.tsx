import { RouterProvider, createBrowserRouter } from 'react-router'
import { setAppNavigator } from '@/kernel/modules/app-navigation'
import { appBase } from '@/sdk/app-base'
import { useModuleTranslationsReady } from '@/kernel/modules/i18n'
import { AppProviders } from './AppProviders'
import { createRoutes } from './routes'

const router = createBrowserRouter(createRoutes(), { basename: appBase() || '/' })
setAppNavigator((to) => void router.navigate(to))

export function App() {
  const ready = useModuleTranslationsReady()
  if (!ready) return null
  return (
    <AppProviders>
      <RouterProvider router={router} />
    </AppProviders>
  )
}
