import { useSearchParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { ExportPanel } from './ExportPanel'
import { HistoryPanel } from './HistoryPanel'
import { InstallPanel } from './InstallPanel'
import { VERSIONS_TAB } from './versioning/tab'
import { VersionsPanel } from './versioning/VersionsPanel'
import { VersionsTabTrigger } from './versioning/VersionsTab'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

const TABS = ['export', 'install', 'history'] as const
type PackagesTab = (typeof TABS)[number] | typeof VERSIONS_TAB
const TAB_PARAM = 'tab'

function parseTab(value: string | null): PackagesTab {
  if (value === VERSIONS_TAB) return VERSIONS_TAB
  return TABS.find((tab) => tab === value) ?? 'export'
}

export function ConfigPackagesPage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <ConfigPackagesSection />
    </RequirePermission>
  )
}

function ConfigPackagesSection() {
  const { t } = useTranslation('config-packages')
  const [searchParams, setSearchParams] = useSearchParams()
  const tab = parseTab(searchParams.get(TAB_PARAM))

  const changeTab = (next: string) =>
    setSearchParams(
      (current) => {
        const params = new URLSearchParams(current)
        params.set(TAB_PARAM, parseTab(next))
        return params
      },
      { replace: true },
    )

  return (
    <Page>
      <PageHeader title={t('page.title')} description={t('page.description')} />
      <Tabs value={tab} onValueChange={changeTab} className="flex min-w-0 flex-col gap-4">
        <TabsList aria-label={t('page.tabsLabel')} className="self-start">
          {TABS.map((value) => (
            <TabsTrigger key={value} value={value}>
              {t(`tabs.${value}`)}
            </TabsTrigger>
          ))}
          <VersionsTabTrigger />
        </TabsList>
        <TabsContent value="export" className="min-w-0">
          <ExportPanel />
        </TabsContent>
        <TabsContent value="install" className="min-w-0">
          <InstallPanel onInstalled={() => changeTab('history')} />
        </TabsContent>
        <TabsContent value="history" className="min-w-0">
          <HistoryPanel />
        </TabsContent>
        <TabsContent value={VERSIONS_TAB} className="min-w-0">
          <VersionsPanel />
        </TabsContent>
      </Tabs>
    </Page>
  )
}
