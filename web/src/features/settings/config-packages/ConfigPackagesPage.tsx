import { useSearchParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { SectionWorkspace, SectionContent, SectionIndex, SectionLink } from '@/components/ui/section-workspace'
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
      { replace: false },
    )

  return (
    <Page>
      <PageHeader title={t('page.title')} description={t('page.description')} />
      <SectionWorkspace value={tab} onValueChange={changeTab}>
        <SectionIndex aria-label={t('page.tabsLabel')} className="self-start">
          {TABS.map((value) => (
            <SectionLink key={value} value={value}>
              {t(`tabs.${value}`)}
            </SectionLink>
          ))}
          <VersionsTabTrigger />
        </SectionIndex>
        <SectionContent value="export" className="min-w-0">
          <ExportPanel />
        </SectionContent>
        <SectionContent value="install" className="min-w-0">
          <InstallPanel onInstalled={() => changeTab('history')} />
        </SectionContent>
        <SectionContent value="history" className="min-w-0">
          <HistoryPanel />
        </SectionContent>
        <SectionContent value={VERSIONS_TAB} className="min-w-0">
          <VersionsPanel />
        </SectionContent>
      </SectionWorkspace>
    </Page>
  )
}
