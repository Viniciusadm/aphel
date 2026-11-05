import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { SectionWorkspace, SectionContent, SectionIndex, SectionLink } from '@/components/ui/section-workspace'
import { PRIVACY_MANAGE, RequirePermission } from '@/sdk'
import { RequestsPanel } from './RequestsPanel'
import { RetentionPanel } from './RetentionPanel'
import { SubjectSearchPanel } from './SubjectSearchPanel'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

const TABS = ['subjects', 'requests', 'retention'] as const
type PrivacyTab = (typeof TABS)[number]
const TAB_PARAM = 'tab'

function parseTab(value: string | null): PrivacyTab {
  return TABS.find((tab) => tab === value) ?? 'subjects'
}

export function PrivacyPage() {
  return (
    <RequirePermission permission={PRIVACY_MANAGE}>
      <PrivacySection />
    </RequirePermission>
  )
}

function PrivacySection() {
  const { t } = useTranslation('privacy')
  const [searchParams, setSearchParams] = useSearchParams()
  const tab = parseTab(searchParams.get(TAB_PARAM))

  const changeTab = (next: string) =>
    setSearchParams(
      (current) => {
        const params = new URLSearchParams()
        params.set(TAB_PARAM, parseTab(next))
        const request = current.get('request')
        if (parseTab(next) === 'requests' && request) params.set('request', request)
        return params
      },
      { replace: false },
    )

  return (
    <Page>
      <PageHeader title={t('page.title')} description={t('page.description')} />
      <SectionWorkspace value={tab} onValueChange={changeTab}>
        <SectionIndex aria-label={t('page.tabsLabel')}>
          {TABS.map((value) => (
            <SectionLink key={value} value={value}>
              {t(`tabs.${value}`)}
            </SectionLink>
          ))}
        </SectionIndex>
        <SectionContent value="subjects" className="min-w-0">
          <SubjectSearchPanel onRequestCreated={() => changeTab('requests')} />
        </SectionContent>
        <SectionContent value="requests" className="min-w-0">
          <RequestsPanel />
        </SectionContent>
        <SectionContent value="retention" className="min-w-0">
          <RetentionPanel />
        </SectionContent>
      </SectionWorkspace>
    </Page>
  )
}
