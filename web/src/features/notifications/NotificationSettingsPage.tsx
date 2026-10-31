import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback'
import { Checkbox } from '@/components/ui/checkbox'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { PushDeviceSetting } from '@/pwa/push/PushDeviceSetting'
import { type NotificationPreference, useNotificationPreferences, useUpdateNotificationPreference } from './api'
import { NOTIFICATION_CHANNELS, orderedNotificationKinds } from './lib/kinds'
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Page } from '@/components/ui/page'

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
    <Page>
      <PageHeader title={t('settings.title')} description={t('settings.description')} />
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
        <Table>
          <caption className="sr-only">{t('settings.caption')}</caption>
          <TableHeader>
            <TableRow>
              <TableHead scope="col" className="text-left">
                {t('settings.kind')}
              </TableHead>
              {NOTIFICATION_CHANNELS.map((channel) => (
                <TableHead key={channel} scope="col" className="w-28 text-center">
                  {t(`channels.${channel}`)}
                </TableHead>
              ))}
            </TableRow>
          </TableHeader>
          <TableBody>
            {orderedNotificationKinds(preferences.data.preferences.map((item) => item.kind)).map((kind) => (
              <TableRow key={kind}>
                <TableHead scope="row" className="text-left font-normal">
                  <span className="grid gap-0.5">
                    <span className="font-medium">{kindLabel(kind)}</span>
                    <span className="text-xs text-muted-foreground">{kindDescription(kind)}</span>
                  </span>
                </TableHead>
                {NOTIFICATION_CHANNELS.map((channel) => {
                  const current = preferences.data.preferences.find(
                    (item) => item.kind === kind && item.channel === channel,
                  )
                  return (
                    <TableCell key={channel} className="text-center">
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
                    </TableCell>
                  )
                })}
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
      <p className="text-xs text-muted-foreground">{t('settings.footnote')}</p>
    </Page>
  )
}
