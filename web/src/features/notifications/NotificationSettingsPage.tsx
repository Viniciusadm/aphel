import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback'
import { Checkbox } from '@/components/ui/checkbox'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { SectionHeader } from '@/features/settings/components/SectionHeader'
import { PushDeviceSetting } from '@/pwa/push/PushDeviceSetting'
import { type NotificationPreference, useNotificationPreferences, useUpdateNotificationPreference } from './api'
import { NOTIFICATION_CHANNELS, orderedNotificationKinds } from './lib/kinds'

export function NotificationSettingsPage() {
  const { t } = useTranslation('notifications')
  const preferences = useNotificationPreferences()
  const update = useUpdateNotificationPreference()
  const kindLabel = (kind: string) => t(`kinds.${kind}.label` as never, { defaultValue: kind })
  const kindDescription = (kind: string) => t(`kinds.${kind}.description` as never, { defaultValue: '' })

  const toggle = (preference: NotificationPreference) => {
    update.mutate(preference, { onError: (error) => notifyError(error, { title: t('settings.saveError') }) })
  }

  return (
    <section className="grid gap-4">
      <SectionHeader title={t('settings.title')} description={t('settings.description')} />
      <PushDeviceSetting />
      {preferences.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-10" />
          <Skeleton className="h-10" />
          <Skeleton className="h-10" />
        </div>
      ) : preferences.isError ? (
        <ErrorState onRetry={() => void preferences.refetch()} />
      ) : (
        <div className="relative overflow-x-auto rounded-lg border">
          <table className="w-full text-sm">
            <caption className="sr-only">{t('settings.caption')}</caption>
            <thead className="bg-muted/50 text-muted-foreground">
              <tr>
                <th scope="col" className="px-3 py-2 text-left font-medium">
                  {t('settings.kind')}
                </th>
                {NOTIFICATION_CHANNELS.map((channel) => (
                  <th key={channel} scope="col" className="w-28 px-3 py-2 text-center font-medium">
                    {t(`channels.${channel}`)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {orderedNotificationKinds(preferences.data.preferences.map((item) => item.kind)).map((kind) => (
                <tr key={kind} className="border-t">
                  <th scope="row" className="px-3 py-2 text-left font-normal">
                    <span className="grid gap-0.5">
                      <span className="font-medium">{kindLabel(kind)}</span>
                      <span className="text-xs text-muted-foreground">{kindDescription(kind)}</span>
                    </span>
                  </th>
                  {NOTIFICATION_CHANNELS.map((channel) => {
                    const current = preferences.data.preferences.find(
                      (item) => item.kind === kind && item.channel === channel,
                    )
                    return (
                      <td key={channel} className="px-3 py-2 text-center">
                        {current ? (
                          <Checkbox
                            aria-label={`${kindLabel(kind)}: ${t(`channels.${channel}`)}`}
                            checked={current.enabled}
                            onCheckedChange={(checked) => toggle({ kind, channel, enabled: checked === true })}
                          />
                        ) : (
                          <span className="text-muted-foreground" aria-label={t('settings.unsupported')}>
                            —
                          </span>
                        )}
                      </td>
                    )
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <p className="text-xs text-muted-foreground">{t('settings.footnote')}</p>
    </section>
  )
}
