import { QueryClientProvider, type QueryClient } from '@tanstack/react-query'
import { useState, type ReactNode } from 'react'
import { I18nextProvider } from 'react-i18next'
import { DemoBanner } from '@/components/feedback/DemoBanner'
import { Toaster } from '@/components/ui/toast'
import { TooltipProvider } from '@/components/ui/tooltip'
import i18n from '@/i18n'
import { createQueryClient } from './query-client'
import { UiLabelsBridge } from './UiLabelsBridge'
import { ThemeProvider } from './theme/ThemeProvider'

interface AppProvidersProps {
  children: ReactNode
  queryClient?: QueryClient
}

export function AppProviders({ children, queryClient }: AppProvidersProps) {
  const [client] = useState(() => queryClient ?? createQueryClient())

  return (
    <I18nextProvider i18n={i18n}>
      <QueryClientProvider client={client}>
        <ThemeProvider>
          <UiLabelsBridge>
            <TooltipProvider>
              <DemoBanner />
              {children}
              <Toaster />
            </TooltipProvider>
          </UiLabelsBridge>
        </ThemeProvider>
      </QueryClientProvider>
    </I18nextProvider>
  )
}
