import { Network, Users } from 'lucide-react'
import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { DEFAULT_READ_ONLY_FIELD_KEYS, RecordDetailPage, type RecordTab } from '@/features/records/detail'
import { RecordLifecycleActions, useProfileTimeline } from '@/modules/contacts/profile'
import { CompanyContactsTab } from './CompanyContactsTab'
import { CompanyGroupTab } from './hierarchy'
import { COMPANIES_PATH, COMPANY_RECORD_TYPE } from './resource'
import '@/modules/contacts/resource'

const READ_ONLY_KEYS = [...DEFAULT_READ_ONLY_FIELD_KEYS, 'created_by']

export function CompanyDetailPage() {
  const { t } = useTranslation('companies')
  const { t: tContacts } = useTranslation('contacts')
  const { id = '' } = useParams()
  const { hasPermission } = useSession()
  const readOnly = !hasPermission('companies.update')
  const timeline = useProfileTimeline(COMPANY_RECORD_TYPE, id)

  const tabs = useCallback(
    (defaults: RecordTab[]): RecordTab[] => {
      const contacts: RecordTab = {
        key: 'related',
        label: tContacts('profile.tabs.contacts'),
        icon: Users,
        content: <CompanyContactsTab companyId={id} />,
      }
      const group: RecordTab = {
        key: 'group',
        label: t('hierarchy.tab'),
        icon: Network,
        content: <CompanyGroupTab companyId={id} readOnly={readOnly} />,
      }
      const [timelineTab, ...rest] = defaults
      return [timelineTab, contacts, group, ...rest.filter((tab) => tab.key !== 'related')]
    },
    [t, tContacts, id, readOnly],
  )

  return (
    <RecordDetailPage
      key={id}
      recordType={COMPANY_RECORD_TYPE}
      recordId={id}
      back={{ to: COMPANIES_PATH, label: t('detail.back') }}
      readOnly={readOnly}
      readOnlyFieldKeys={READ_ONLY_KEYS}
      timeline={timeline}
      tabs={tabs}
      actions={(record) => (
        <RecordLifecycleActions
          recordType={COMPANY_RECORD_TYPE}
          record={record}
          listPath={COMPANIES_PATH}
          archivePermission="companies.archive"
          deletePermission="companies.delete"
        />
      )}
    />
  )
}
