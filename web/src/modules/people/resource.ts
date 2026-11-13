import { Users } from 'lucide-react'
import { registerRecordResource } from '@/sdk'
export const CONTACT_RECORD_TYPE = 'people.person'
export const CONTACTS_PATH = '/people'

export const contactResource = registerRecordResource({
  relatable: true,
  icon: Users,
  checkIn: true,
  recordType: CONTACT_RECORD_TYPE,
  createPermission: 'people.create',
  updatePermission: 'people.update',
  basePath: '/v1/people',
  summaryFields: ['email', 'phone', 'mobile', 'company_id', 'job_title', 'owner_id', 'lifecycle_stage'],
  titleField: 'name',
  subtitleField: 'email',
  defaultColumns: ['name', 'email', 'phone', 'company_id', 'job_title', 'owner_id'],
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

export { CONTACT_COMPANY_ROLES, type ContactCompanyRole } from '@/sdk'
