import { Network, Users } from 'lucide-react'
import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { DEFAULT_READ_ONLY_FIELD_KEYS, RecordDetailPage, type RecordTab } from '@/sdk'
import { RecordLifecycleActions, useProfileTimeline } from '@/modules/people/profile'
import { CompanyContactsTab } from './CompanyContactsTab'
import { CompanyGroupTab } from './hierarchy'
import { COMPANIES_PATH, COMPANY_RECORD_TYPE } from './resource'
import '@/modules/people/resource'

const READ_ONLY_KEYS = [...DEFAULT_READ_ONLY_FIELD_KEYS, 'created_by']

export function CompanyDetailPage() {
  const { t } = useTranslation('organizations')
  const { t: tContacts } = useTranslation('people')
  const { id = '' } = useParams()
  const { hasPermission } = useSession()
  const readOnly = !hasPermission('organizations.update')
  const timeline = useProfileTimeline(COMPANY_RECORD_TYPE, id)

  const tabs = useCallback(
    (defaults: RecordTab[]): RecordTab[] => {
      const people: RecordTab = {
        key: 'related',
        label: tContacts('profile.tabs.people'),
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
      return [timelineTab, people, group, ...rest.filter((tab) => tab.key !== 'related')]
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
          archivePermission="organizations.archive"
          deletePermission="organizations.delete"
        />
      )}
    />
  )
}
