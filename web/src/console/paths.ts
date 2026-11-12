export const SIGN_IN_PATH = '/sign-in'
export const INVITATION_PATH = '/invite'
export const TENANTS_PATH = '/'
export const NEW_TENANT_PATH = '/tenants/new'
export const OPERATORS_PATH = '/operators'
export const AUDIT_PATH = '/audit'
export const PASSWORD_PATH = '/password'
export const SESSION_REQUIRED_CODE = 'console_session_required'

export function tenantPath(slug: string): string {
  return `/tenants/${slug}`
}

export function tenantPluginsPath(slug: string): string {
  return `${tenantPath(slug)}/plugins`
}

export function isPublicPath(path: string): boolean {
  return path === SIGN_IN_PATH || path.startsWith(`${INVITATION_PATH}/`)
}
