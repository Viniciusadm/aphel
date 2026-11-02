import { registerRecordResource } from '@/features/records/resources'

export const CONTACT_RECORD_TYPE = 'contact'
export const CONTACTS_PATH = '/contacts'

export const contactResource = registerRecordResource({
  recordType: CONTACT_RECORD_TYPE,
  createPermission: 'contacts.create',
  updatePermission: 'contacts.update',
  basePath: '/v1/contacts',
  summaryFields: ['email', 'phone', 'mobile', 'company_id', 'owner_id', 'lifecycle_stage'],
  titleField: 'name',
  subtitleField: 'email',
  detailPath: (id) => `${CONTACTS_PATH}/${encodeURIComponent(id)}`,
})

export const CONTACT_CREATE_LAYOUT = [
  'first_name',
  'last_name',
  'email',
  'phone',
  'mobile',
  'job_title',
  'company_id',
  'owner_id',
]

export const CONTACT_COMPANY_ROLES = ['decision_maker', 'buyer', 'influencer', 'user', 'other'] as const

export type ContactCompanyRole = (typeof CONTACT_COMPANY_ROLES)[number]
