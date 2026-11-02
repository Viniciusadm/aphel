import { useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import { configPackagesApi, configPackagesKeys } from '@/modules/config_packages/ui/api'
import type { CompleteBody, OnboardingState, SaveAnswersBody } from './types'

export {
  PLAN_OUTDATED_CODE,
  type ApplyResult,
  type PackagePlan,
  type PackageTemplate,
  type PlanItem,
} from '@/modules/config_packages/ui/api'

export const onboardingKeys = {
  all: ['onboarding'] as const,
  state: () => [...onboardingKeys.all, 'state'] as const,
  plan: (template: string) => [...onboardingKeys.all, 'plan', template] as const,
}

export const onboardingApi = {
  get: () => api.get<OnboardingState>('/v1/onboarding'),
  saveAnswers: (body: SaveAnswersBody) => api.put<OnboardingState>('/v1/onboarding/answers', body),
  complete: (body: CompleteBody) => api.post<OnboardingState>('/v1/onboarding/complete', body),
  skip: () => api.post<OnboardingState>('/v1/onboarding/skip'),
  reopen: () => api.post<OnboardingState>('/v1/onboarding/reopen'),
  plan: (template: string) => configPackagesApi.plan({ template }),
  apply: (template: string, fingerprint: string) => configPackagesApi.apply({ template }, fingerprint, {}),
}

export function useOnboardingState(enabled = true) {
  return useQuery({
    queryKey: onboardingKeys.state(),
    queryFn: onboardingApi.get,
    enabled,
    staleTime: 30_000,
  })
}

export function useTemplates(enabled = true) {
  return useQuery({
    queryKey: configPackagesKeys.templates(),
    queryFn: configPackagesApi.templates,
    enabled,
    staleTime: 5 * 60_000,
  })
}

export function usePlan(template: string | null) {
  return useQuery({
    queryKey: onboardingKeys.plan(template ?? ''),
    queryFn: () => onboardingApi.plan(template ?? ''),
    enabled: template !== null,
    staleTime: 0,
    gcTime: 0,
  })
}
