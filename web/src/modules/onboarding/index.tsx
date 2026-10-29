import { Rocket } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { defineModule } from '@/kernel/modules/define'
import { OnboardingGate } from './OnboardingGate'

const onboardingModule = defineModule({
  key: 'onboarding',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes(
      [
        {
          path: 'onboarding',
          lazy: async () => ({ Component: (await import('./OnboardingPage')).OnboardingPage }),
        },
      ],
      { layout: 'bare' },
    )
    app.settings('data', {
      group: 'transfer',
      path: 'onboarding',
      label: 'settings:tabs.onboarding',
      description: 'settings:items.onboarding.description',
      icon: Rocket,
      permission: SETTINGS_MANAGE,
      lazy: async () => ({ Component: (await import('./OnboardingSettingsPage')).OnboardingSettingsPage }),
    })
    app.slot('app.shell', { key: 'gate', render: OnboardingGate })
  },
})

export default onboardingModule
