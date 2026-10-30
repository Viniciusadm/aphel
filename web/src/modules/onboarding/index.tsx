import { Rocket } from 'lucide-react'
import { Navigate } from 'react-router'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { defineModule } from '@/kernel/modules/define'
import { OnboardingGate } from './OnboardingGate'
import { applyRecommendedTemplate } from './setup/applyRecommendedTemplate'
import { TemplateStep } from './setup/TemplateStep'
import { useOnboardingPending } from './setup/usePending'

const onboardingModule = defineModule({
  key: 'onboarding',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes([{ path: 'onboarding', element: <Navigate to="/setup" replace /> }], { layout: 'bare' })
    app.setupStep({
      key: 'template',
      label: 'onboarding:setup.step',
      order: 50,
      usePending: useOnboardingPending,
      render: TemplateStep,
    })
    app.setupTask({
      key: 'template',
      progressLabel: 'onboarding:setup.task.progress',
      failureLabel: 'onboarding:setup.task.failed',
      order: 50,
      run: applyRecommendedTemplate,
    })
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
