import type { components } from '@/api'

type Schemas = components['schemas']

export type ShareGranteeType = 'user' | 'team'
export type ShareAccess = 'read' | 'edit'

export type ShareGrantee = Omit<Schemas['ShareGranteeResponse'], 'type'> & { type: ShareGranteeType }
export type RecordShare = Omit<Schemas['RecordShareResponse'], 'grantee' | 'access'> & {
  grantee: ShareGrantee
  access: ShareAccess
}
export type RecordShares = Omit<Schemas['RecordSharesResponse'], 'shares'> & { shares: RecordShare[] }
export type CreateRecordShareRequest = Omit<Schemas['CreateRecordShareRequest'], 'grantee_type' | 'access'> & {
  grantee_type: ShareGranteeType
  access: ShareAccess
}
export type UpdateRecordShareRequest = Omit<Schemas['UpdateRecordShareRequest'], 'access'> & { access: ShareAccess }
