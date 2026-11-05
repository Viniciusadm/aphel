import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import type { SetupStepContribution } from '@/kernel/modules/define'
import type { Contribution } from '@/kernel/modules/registry'

export const DRAFT_STEPS = ['organization', 'business', 'features', 'account'] as const
export type DraftStep = (typeof DRAFT_STEPS)[number]

export function isDraftStep(value: unknown): value is DraftStep {
  return typeof value === 'string' && (DRAFT_STEPS as readonly string[]).includes(value)
}

export function contributedStepKey(item: Contribution<SetupStepContribution>): string {
  return `${item.module}:${item.value.key}`
}

export function useStepLabel(): (key: string) => string {
  const { t } = useTranslation()
  return useCallback((key: string) => String((t as unknown as (key: string) => string)(key)), [t])
}
