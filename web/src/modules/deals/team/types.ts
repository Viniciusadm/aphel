import type { components } from '@/modules/deals/api-types'

type Schemas = components['schemas']

export type DealMemberUser = Schemas['UserReferenceResponse']
export type DealMemberRoleSummary = Schemas['DealMemberRoleSummaryResponse']
export type DealMember = Schemas['DealMemberResponse']
export type DealTeamOwner = Schemas['DealTeamOwnerResponse']
export type DealTeam = Schemas['DealTeamResponse']
export type AddDealMemberRequest = Schemas['AddDealMemberRequest']
export type UpdateDealMemberRequest = Schemas['UpdateDealMemberRequest']
export type DealMemberRole = Schemas['DealMemberRoleResponse']
export type CreateDealMemberRoleRequest = Schemas['CreateDealMemberRoleRequest']
export type UpdateDealMemberRoleRequest = Schemas['UpdateDealMemberRoleRequest']

export const DEAL_MEMBER_EVENTS = ['deal.member_added', 'deal.member_updated', 'deal.member_removed'] as const

export type DealMemberEventType = (typeof DEAL_MEMBER_EVENTS)[number]
