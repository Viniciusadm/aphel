import { useTranslation } from 'react-i18next'
import { SectionWorkspace, SectionContent, SectionIndex, SectionLink } from '@/components/ui/section-workspace'
import { RequirePermission, SETTINGS_MANAGE } from '@/sdk'
import { TemplatesPanel } from '../components/TemplatesPanel'
import { BlocklistPanel } from './BlocklistPanel'
import { PreferencesPanel } from './PreferencesPanel'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'


export function EmailSettingsPage() {
  const { t } = useTranslation('email')

  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <Page>
        <PageHeader title={t('settings.title')} description={t('settings.description')} />
        <SectionWorkspace defaultValue="templates">
          <SectionIndex>
            <SectionLink value="templates">{t('settings.tabs.templates')}</SectionLink>
            <SectionLink value="blocklist">{t('settings.tabs.blocklist')}</SectionLink>
            <SectionLink value="preferences">{t('settings.tabs.preferences')}</SectionLink>
          </SectionIndex>
          <SectionContent value="templates">
            <TemplatesPanel scope="organization" />
          </SectionContent>
          <SectionContent value="blocklist">
            <BlocklistPanel />
          </SectionContent>
          <SectionContent value="preferences" forceMount className="data-[state=inactive]:hidden">
            <PreferencesPanel />
          </SectionContent>
        </SectionWorkspace>
      </Page>
    </RequirePermission>
  )
}
