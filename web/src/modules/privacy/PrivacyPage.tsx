import { useTranslation } from 'react-i18next'
import { useSearchParams } from 'react-router'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { PRIVACY_MANAGE } from '@/features/settings/lib/permissions'
import { RequestsPanel } from './RequestsPanel'
import { RetentionPanel } from './RetentionPanel'
import { SubjectSearchPanel } from './SubjectSearchPanel'
import { PageHeader } from '@/components/ui/page-header'

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
      { replace: true },
    )

  return (
    <section className="flex min-w-0 flex-col gap-4">
      <PageHeader title={t('page.title')} description={t('page.description')} />
      <Tabs value={tab} onValueChange={changeTab} className="flex min-w-0 flex-col gap-4">
        <TabsList aria-label={t('page.tabsLabel')}>
          {TABS.map((value) => (
            <TabsTrigger key={value} value={value}>
              {t(`tabs.${value}`)}
            </TabsTrigger>
          ))}
        </TabsList>
        <TabsContent value="subjects" className="min-w-0">
          <SubjectSearchPanel onRequestCreated={() => changeTab('requests')} />
        </TabsContent>
        <TabsContent value="requests" className="min-w-0">
          <RequestsPanel />
        </TabsContent>
        <TabsContent value="retention" className="min-w-0">
          <RetentionPanel />
        </TabsContent>
      </Tabs>
    </section>
  )
}
