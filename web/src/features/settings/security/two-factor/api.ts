import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api, isApiError, type components } from '@/api'
import { SESSION_QUERY_KEY } from '@/app/session/session-query'

type Schemas = components['schemas']

export type TwoFactorStatus = Schemas['TwoFactorStatusResponse']
export type TwoFactorSetup = Schemas['TwoFactorSetupResponse']
export type RecoveryCodes = Schemas['RecoveryCodesResponse']
export type TwoFactorPolicy = Schemas['TwoFactorPolicyDto']
export type TwoFactorPolicyState = Schemas['TwoFactorPolicyResponse']
export type DisableTwoFactorBody = Schemas['DisableTwoFactorRequest']
export type RegenerateRecoveryCodesBody = Schemas['RegenerateRecoveryCodesRequest']

export const TWO_FACTOR_POLICIES: TwoFactorPolicy[] = ['none', 'admins', 'everyone']

export const INVALID_TWO_FACTOR_CODE = 'invalid_two_factor_code'
export const INVALID_CURRENT_PASSWORD = 'invalid_current_password'
export const TWO_FACTOR_NOT_ENABLED = 'two_factor_not_enabled'
export const TWO_FACTOR_SETUP_REQUIRED = 'two_factor_setup_required'

export const twoFactorKeys = {
  all: ['two-factor'] as const,
  status: () => [...twoFactorKeys.all, 'status'] as const,
  setup: () => [...twoFactorKeys.all, 'setup'] as const,
  policy: () => [...twoFactorKeys.all, 'policy'] as const,
}

function isSetupQuery(queryKey: readonly unknown[]): boolean {
  return queryKey[0] === twoFactorKeys.all[0] && queryKey[1] === 'setup'
}

export const twoFactorApi = {
  status: () => api.get<TwoFactorStatus>('/v1/two-factor'),
  setup: () => api.post<TwoFactorSetup>('/v1/two-factor/setup'),
  confirm: (code: string) =>
    api.post<RecoveryCodes>('/v1/two-factor/confirm', { code } satisfies Schemas['ConfirmTwoFactorRequest']),
  disable: (body: DisableTwoFactorBody) => api.post<void>('/v1/two-factor/disable', body),
  regenerate: (body: RegenerateRecoveryCodesBody) => api.post<RecoveryCodes>('/v1/two-factor/recovery-codes', body),
  policy: () => api.get<TwoFactorPolicyState>('/v1/organization/two-factor-policy'),
  updatePolicy: (policy: TwoFactorPolicy) =>
    api.put<TwoFactorPolicyState>('/v1/organization/two-factor-policy', {
      policy,
    } satisfies Schemas['UpdateTwoFactorPolicyRequest']),
}

export function useTwoFactorStatus() {
  return useQuery({ queryKey: twoFactorKeys.status(), queryFn: twoFactorApi.status })
}

export function useTwoFactorPolicy(enabled: boolean) {
  return useQuery({
    queryKey: twoFactorKeys.policy(),
    queryFn: twoFactorApi.policy,
    enabled,
    retry: (count, error) => !(isApiError(error) && error.status === 403) && count < 2,
  })
}

export function useRefreshAfterTwoFactorChange() {
  const queryClient = useQueryClient()
  return async () => {
    await queryClient.invalidateQueries({ queryKey: SESSION_QUERY_KEY })
    await queryClient.invalidateQueries({
      predicate: (query) => query.queryKey[0] !== SESSION_QUERY_KEY[0] && !isSetupQuery(query.queryKey),
    })
  }
}

export function hasErrorCode(error: unknown, code: string): boolean {
  return isApiError(error) && error.code === code
}

export function recoveryCodesFile(heading: string, codes: string[]): string {
  return [heading, '', ...codes, ''].join('\n')
}
