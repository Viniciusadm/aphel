import { useTranslation } from 'react-i18next'
import { ErrorState } from '@/components/ui/error-state'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Skeleton } from '@/components/ui/skeleton'
import { usePersonalPreferences } from '@/features/settings/navigation/api'
import { useSaveOrganizationKeybindings } from './api'
import { KeybindingsEditor } from './KeybindingsEditor'

export function OrganizationKeyboardPage() {
  const { t } = useTranslation('keybindings')
  const preferences = usePersonalPreferences()
  const keybindings = useSaveOrganizationKeybindings()

  if (preferences.isPending) return <Skeleton className="h-64" />
  if (preferences.isError) return <ErrorState onRetry={() => void preferences.refetch()} />

  return (
    <Page>
      <PageHeader title={t('organization.title')} description={t('organization.description')} />
      <div className="mt-6">
        <KeybindingsEditor
          layer="organization"
          overrides={preferences.data.keybindings_default}
          pending={keybindings.isPending}
          onSave={keybindings.save}
        />
      </div>
    </Page>
  )
}
