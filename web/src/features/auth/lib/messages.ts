export type AuthMessageKey =
  | 'validation.required'
  | 'validation.email'
  | 'validation.passwordLength'
  | 'validation.nameLength'
  | 'validation.invalid'
  | 'errors.invalidCredentials'
  | 'errors.wrongPassword'
  | 'errors.emailTaken'
  | 'errors.rateLimited'
  | 'errors.loginLocked'
  | 'errors.loginLockedUnknown'
  | 'errors.network'
  | 'errors.generic'
  | 'errors.notAMember'
  | 'errors.ssoRequired'
  | 'errors.ssoFailed'
  | 'errors.ssoNoAccount'

export function asAuthMessageKey(message: string | undefined): AuthMessageKey | undefined {
  return message ? (message as AuthMessageKey) : undefined
}
