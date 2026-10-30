export const BUSINESS_TYPES = ['freelancer', 'agency', 'real_estate', 'consultancy', 'saas', 'sales', 'other'] as const
export const OFFERINGS = ['product', 'service', 'recurring'] as const
export const TEAM_SIZES = ['solo', 'small', 'medium', 'large'] as const

export type BusinessType = (typeof BUSINESS_TYPES)[number]
export type Offering = (typeof OFFERINGS)[number]
export type TeamSize = (typeof TEAM_SIZES)[number]

export interface BusinessAnswers {
  business_type: BusinessType | null
  offering: Offering[]
  team_size: TeamSize | null
}

export const EMPTY_BUSINESS: BusinessAnswers = { business_type: null, offering: [], team_size: null }

function isOneOf<T extends string>(values: readonly T[], value: unknown): value is T {
  return typeof value === 'string' && (values as readonly string[]).includes(value)
}

export function parseBusiness(value: unknown): BusinessAnswers {
  if (!value || typeof value !== 'object') return EMPTY_BUSINESS
  const raw = value as Record<string, unknown>
  return {
    business_type: isOneOf(BUSINESS_TYPES, raw.business_type) ? raw.business_type : null,
    offering: Array.isArray(raw.offering) ? raw.offering.filter((item): item is Offering => isOneOf(OFFERINGS, item)) : [],
    team_size: isOneOf(TEAM_SIZES, raw.team_size) ? raw.team_size : null,
  }
}
