import i18n from '@/i18n'
import type { SetupTaskResult } from '@/kernel/modules/define'
import { onboardingApi } from '../api'

export async function applyRecommendedTemplate(): Promise<SetupTaskResult | null> {
  const state = await onboardingApi.get()
  if (state.status !== 'pending') return null
  const templateKey = state.recommendation?.recommended.key
  if (!templateKey) {
    await onboardingApi.complete({ template_key: null, install_id: null })
    return null
  }
  const plan = await onboardingApi.plan(templateKey)
  if (!plan.valid) throw new Error(`template ${templateKey} is not installable`)
  const { install } = await onboardingApi.apply(templateKey, plan.fingerprint)
  await onboardingApi.complete({ template_key: templateKey, install_id: install.id })
  return {
    summary: i18n.t('onboarding:setup.task.applied', {
      name: plan.package?.name ?? templateKey,
      count: plan.summary.create,
    }),
  }
}
