import { Building2 } from 'lucide-react'
import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { useParams } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { DEFAULT_READ_ONLY_FIELD_KEYS, RecordDetailPage, type RecordTab } from '@/sdk'
import { ContactCompaniesTab } from './ContactCompaniesTab'
import { RecordLifecycleActions, useProfileTimeline } from './profile'
import { CONTACTS_PATH, CONTACT_RECORD_TYPE } from './resource'
import '@/modules/organizations/resource'

const READ_ONLY_KEYS = [...DEFAULT_READ_ONLY_FIELD_KEYS, 'name', 'created_by']

export function ContactDetailPage() {
  const { t } = useTranslation('people')
  const { id = '' } = useParams()
  const { hasPermission } = useSession()
  const readOnly = !hasPermission('people.update')
  const timeline = useProfileTimeline(CONTACT_RECORD_TYPE, id)

  const tabs = useCallback(
    (defaults: RecordTab[]): RecordTab[] => {
      const organizations: RecordTab = {
        key: 'related',
        group: 'relationships',
        label: t('profile.tabs.organizations'),
        icon: Building2,
        content: <ContactCompaniesTab contactId={id} readOnly={readOnly} />,
      }
      const [timelineTab, ...rest] = defaults
      return [timelineTab, ...rest.map((tab) => (tab.key === 'related' ? organizations : tab))]
    },
    [t, id, readOnly],
  )

  return (
    <RecordDetailPage
      key={id}
      recordType={CONTACT_RECORD_TYPE}
      recordId={id}
      back={{ to: CONTACTS_PATH, label: t('detail.back') }}
      readOnly={readOnly}
      readOnlyFieldKeys={READ_ONLY_KEYS}
      timeline={timeline}
      tabs={tabs}
      actions={(record) => (
        <RecordLifecycleActions
          recordType={CONTACT_RECORD_TYPE}
          record={record}
          listPath={CONTACTS_PATH}
          archivePermission="people.archive"
          deletePermission="people.delete"
        />
      )}
    />
  )
}
