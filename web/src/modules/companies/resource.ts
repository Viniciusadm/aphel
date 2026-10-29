import { registerRecordResource } from '@/features/records/resources'

export const COMPANY_RECORD_TYPE = 'company'
export const COMPANIES_PATH = '/companies'

export const companyResource = registerRecordResource({
  recordType: COMPANY_RECORD_TYPE,
  basePath: '/v1/companies',
  titleField: 'name',
  subtitleField: 'domain',
  detailPath: (id) => `${COMPANIES_PATH}/${encodeURIComponent(id)}`,
})

export const COMPANY_CREATE_LAYOUT = ['name', 'domain', 'website', 'email', 'phone', 'owner_id']
