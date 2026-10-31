import { useId } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { ErrorState } from '@/components/ui/error-state'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { useCommandPalette } from '../context'
import { usePreferences, useUpdatePreferences } from './api'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function PreferencesPage() {
  const { t } = useTranslation('command-palette')
  const preferences = usePreferences()
  const update = useUpdatePreferences()
  const { openHelp } = useCommandPalette()
  const shortcutsId = useId()

  const setShortcuts = (enabled: boolean) => {
    update.mutate(
      { keyboard_shortcuts: enabled },
      {
        onSuccess: () => toast({ title: t('preferences.saved'), variant: 'success' }),
        onError: (error) => notifyError(error, { title: t('preferences.saveError') }),
      },
    )
  }

  return (
    <Page>
      <PageHeader title={t('preferences.title')} description={t('preferences.description')} />
      {preferences.isPending ? (
        <Skeleton className="h-20" />
      ) : preferences.isError ? (
        <ErrorState onRetry={() => void preferences.refetch()} />
      ) : (
        <div className="flex flex-wrap items-start gap-3 rounded-lg border p-4">
          <Checkbox
            id={shortcutsId}
            checked={preferences.data.keyboard_shortcuts}
            disabled={update.isPending}
            onCheckedChange={(checked) => setShortcuts(checked === true)}
            aria-describedby={`${shortcutsId}-description`}
            className="mt-0.5"
          />
          <div className="grid min-w-0 flex-1 gap-1">
            <Label htmlFor={shortcutsId}>{t('preferences.shortcuts.label')}</Label>
            <p id={`${shortcutsId}-description`} className="text-sm text-muted-foreground">
              {t('preferences.shortcuts.description')}
            </p>
          </div>
          <Button variant="outline" size="sm" onClick={openHelp}>
            {t('preferences.shortcuts.showList')}
          </Button>
        </div>
      )}
    </Page>
  )
}
