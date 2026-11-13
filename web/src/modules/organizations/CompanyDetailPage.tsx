import { Network, Users } from 'lucide-react'
import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useParams } from 'react-router'
import { useIsModuleActive } from '@/kernel/modules/registry'
import { useSession } from '@/app/session/session-context'
import { DEFAULT_READ_ONLY_FIELD_KEYS, RecordDetailPage, type RecordTab } from '@/sdk'
import {RecordLifecycleActions, useProfileTimeline} from '@/sdk'
import { CompanyContactsTab } from './CompanyContactsTab'
import { CompanyGroupTab } from './hierarchy'
import { COMPANIES_PATH, COMPANY_RECORD_TYPE } from './resource'

const READ_ONLY_KEYS = [...DEFAULT_READ_ONLY_FIELD_KEYS, 'created_by']

export function CompanyDetailPage() {
  const { t } = useTranslation('organizations')
  const { t: tContacts } = useTranslation('people')
  const { id = '' } = useParams()
  const { hasPermission } = useSession()
  const readOnly = !hasPermission('organizations.update')
  const timeline = useProfileTimeline(COMPANY_RECORD_TYPE, id)
  const peopleActive = useIsModuleActive('people')

  const tabs = useCallback(
    (defaults: RecordTab[]): RecordTab[] => {
      const people: RecordTab = {
        key: 'related',
        group: 'relationships',
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
      const timelineTab = defaults.find((tab) => tab.key === 'timeline')
      const rest = defaults.filter((tab) => tab.key !== 'timeline' && tab.key !== 'related')
      return [...(timelineTab ? [timelineTab] : []), ...(peopleActive ? [people] : []), group, ...rest]
    },
    [t, tContacts, id, readOnly, peopleActive],
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
