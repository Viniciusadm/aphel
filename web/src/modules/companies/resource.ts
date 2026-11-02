import { Building2 } from 'lucide-react'
import { registerRecordResource } from '@/features/records/resources'

export const COMPANY_RECORD_TYPE = 'companies.company'
export const COMPANIES_PATH = '/companies'

export const companyResource = registerRecordResource({
  relatable: true,
  icon: Building2,
  checkIn: true,
  recordType: COMPANY_RECORD_TYPE,
  createPermission: 'companies.create',
  updatePermission: 'companies.update',
  basePath: '/v1/companies',
  summaryFields: ['domain', 'website', 'email', 'phone', 'owner_id', 'industry'],
  titleField: 'name',
  subtitleField: 'domain',
  detailPath: (id) => `${COMPANIES_PATH}/${encodeURIComponent(id)}`,
})

export const COMPANY_CREATE_LAYOUT = ['name', 'domain', 'website', 'email', 'phone', 'owner_id']
