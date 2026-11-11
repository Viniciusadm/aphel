import { RouterProvider } from 'react-router'
import { AppProviders } from '@/app/AppProviders'
import { consoleRouter } from './router'

export function ConsoleApp() {
  return (
    <AppProviders>
      <RouterProvider router={consoleRouter} />
    </AppProviders>
  )
}
