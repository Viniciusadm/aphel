import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback'
import { ErrorState } from '@/components/ui/error-state'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Skeleton } from '@/components/ui/skeleton'
import { ToggleField } from '@/components/ui/toggle-field'
import { usePersonalPreferences, useUpdatePersonalPreferences } from '@/features/settings/navigation/api'
import { chordText, useActionKeys } from '@/kernel/keybindings'
import { useSaveUserKeybindings } from './api'
import { KeybindingsEditor } from './KeybindingsEditor'

export function UserKeyboardPage() {
  const { t } = useTranslation('keybindings')
  const preferences = usePersonalPreferences()
  const update = useUpdatePersonalPreferences()
  const keybindings = useSaveUserKeybindings()
  const paletteKeys = useActionKeys('core.palette')

  if (preferences.isPending) return <Skeleton className="h-64" />
  if (preferences.isError) return <ErrorState onRetry={() => void preferences.refetch()} />

  return (
    <Page>
      <PageHeader title={t('user.title')} description={t('user.description')} />
      <div className="mt-6 grid gap-8">
        <div className="rounded-lg border p-4">
          <ToggleField
            type="switch"
            label={t('user.enabled')}
            description={t('user.enabledHint', { keys: paletteKeys.map((chord) => chordText(chord)).join(' / ') })}
            checked={preferences.data.keyboard_shortcuts}
            disabled={update.isPending}
            onCheckedChange={(checked) =>
              update.mutate({ keyboard_shortcuts: checked }, { onError: (error) => notifyError(error, { title: t('saveError') }) })
            }
          />
        </div>
        <KeybindingsEditor
          layer="user"
          overrides={preferences.data.keybindings}
          pending={keybindings.isPending}
          onSave={keybindings.save}
        />
      </div>
    </Page>
  )
}
