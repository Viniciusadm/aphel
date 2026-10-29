import type { BusinessType, Offering, TeamSize } from '../types'

export const BUSINESS_TYPES: BusinessType[] = ['real_estate', 'agency', 'consulting', 'software', 'b2b_sales', 'other']
export const OFFERINGS: Offering[] = ['product', 'service', 'recurring']
export const TEAM_SIZES: TeamSize[] = ['solo', 'small', 'medium', 'large']

export const STEPS = ['business', 'offering', 'team', 'recommendation', 'preview'] as const
export type Step = (typeof STEPS)[number]

export const SCRATCH = 'scratch'
