import { api, type components } from '@/api'

type Schemas = components['schemas']

export type TwoFactorChallengeRequest = Schemas['TwoFactorChallengeRequest']

export const INVALID_TWO_FACTOR_CODE = 'invalid_two_factor_code'
export const TWO_FACTOR_ATTEMPTS_EXCEEDED = 'two_factor_attempts_exceeded'

export function submitTwoFactorChallenge(code: string) {
  return api.post<Schemas['MeResponse']>('/v1/auth/two-factor/challenge', {
    code,
  } satisfies TwoFactorChallengeRequest)
}
