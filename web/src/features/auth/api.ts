import { api, isApiError, type components } from '@/api'

type Schemas = components['schemas']

export type MeResponse = Schemas['MeResponse']
export type LoginResponse = Schemas['LoginResponse']
export type TwoFactorRequiredResponse = Schemas['TwoFactorRequiredResponse']
export type InvitationResponse = Schemas['InvitationResponse']
export type LoginRequest = Schemas['LoginRequest']
export type ResetPasswordRequest = Schemas['ResetPasswordRequest']
export type AcceptInvitationRequest = Schemas['AcceptInvitationRequest']
export type SsoOption = Schemas['SsoOptionResponse']
export type SsoDiscoverResponse = Schemas['SsoDiscoverResponse']

const AUTH_PATH = '/v1/auth'

export async function fetchSession(): Promise<MeResponse | null> {
  try {
    return await api.get<MeResponse>(`${AUTH_PATH}/me`)
  } catch (error) {
    if (isApiError(error) && error.status === 401) return null
    throw error
  }
}

export function isTwoFactorRequired(response: LoginResponse): response is TwoFactorRequiredResponse {
  return 'two_factor_required' in response && response.two_factor_required
}

export function login(body: LoginRequest) {
  return api.post<LoginResponse>(`${AUTH_PATH}/login`, body)
}

export async function discoverSso(email: string): Promise<SsoOption[]> {
  const response = await api.post<SsoDiscoverResponse>(`${AUTH_PATH}/sso/discover`, { email })
  return response.data
}

export function logout() {
  return api.post<void>(`${AUTH_PATH}/logout`)
}

export function logoutAll() {
  return api.post<void>(`${AUTH_PATH}/logout-all`)
}

export function verifyEmail(token: string) {
  return api.post<void>(`${AUTH_PATH}/verify-email`, { token } satisfies Schemas['TokenRequest'])
}

export function resendVerification() {
  return api.post<void>(`${AUTH_PATH}/resend-verification`)
}

export function forgotPassword(body: Schemas['ForgotPasswordRequest']) {
  return api.post<void>(`${AUTH_PATH}/forgot-password`, body)
}

export function resetPassword(body: ResetPasswordRequest) {
  return api.post<void>(`${AUTH_PATH}/reset-password`, body)
}

export function fetchInvitation(token: string) {
  return api.post<InvitationResponse>(`${AUTH_PATH}/invitation`, { token } satisfies Schemas['TokenRequest'])
}

export function acceptInvitation(body: AcceptInvitationRequest) {
  return api.post<LoginResponse>(`${AUTH_PATH}/accept-invitation`, body)
}
