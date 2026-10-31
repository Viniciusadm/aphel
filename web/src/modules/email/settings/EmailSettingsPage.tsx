import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { TemplatesPanel } from '../components/TemplatesPanel'
import { BlocklistPanel } from './BlocklistPanel'
import { PreferencesPanel } from './PreferencesPanel'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

type Section = 'templates' | 'blocklist' | 'preferences'

export function EmailSettingsPage() {
  const { t } = useTranslation('email')
  const [section, setSection] = useState<Section>('templates')

  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <Page>
        <PageHeader title={t('settings.title')} description={t('settings.description')} />
        <Tabs value={section} onValueChange={(value) => setSection(value as Section)} className="grid gap-4">
          <TabsList>
            <TabsTrigger value="templates">{t('settings.tabs.templates')}</TabsTrigger>
            <TabsTrigger value="blocklist">{t('settings.tabs.blocklist')}</TabsTrigger>
            <TabsTrigger value="preferences">{t('settings.tabs.preferences')}</TabsTrigger>
          </TabsList>
          <TabsContent value="templates">
            <TemplatesPanel scope="organization" />
          </TabsContent>
          <TabsContent value="blocklist">
            <BlocklistPanel />
          </TabsContent>
          <TabsContent value="preferences" forceMount className="data-[state=inactive]:hidden">
            <PreferencesPanel />
          </TabsContent>
        </Tabs>
      </Page>
    </RequirePermission>
  )
}
