export const CONTACT_COMPANY_ROLES = ['decision_maker', 'buyer', 'influencer', 'user', 'other'] as const

export type ContactCompanyRole = (typeof CONTACT_COMPANY_ROLES)[number]
