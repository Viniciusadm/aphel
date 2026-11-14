import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Notice } from '@/components/ui/notice'
import { SectionWorkspace, SectionContent, SectionIndex, SectionLink } from '@/components/ui/section-workspace'
import { MAILBOXES_PATH, RequirePermission, SETTINGS_MANAGE } from '@/sdk'
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
        <Notice>
          <p>
            {t('settings.scope')}{' '}
            <Link to={MAILBOXES_PATH} className="font-medium underline underline-offset-4">
              {t('settings.mailboxesLink')}
            </Link>
            .
          </p>
          <p>
            {t('settings.system')}{' '}
            <Link to="/settings/outgoing-email" className="font-medium underline underline-offset-4">
              {t('settings.systemLink')}
            </Link>
            .
          </p>
        </Notice>
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
