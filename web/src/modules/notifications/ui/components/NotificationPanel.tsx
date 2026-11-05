import { SegmentedControl } from '@/components/ui/segmented-control'
import { CheckCheck, Settings } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { useRecordFormatting } from '@/sdk'
import { cn } from '@/lib/utils'
import {
  NOTIFICATIONS_PAGE_SIZE,
  type Notification,
  useMarkAllNotificationsRead,
  useMarkNotificationRead,
  useNotifications,
} from '@/modules/notifications/ui/api'
import { isDownloadLink, notificationHeadline, notificationSubject } from '@/modules/notifications/ui/lib/content'
import { relativeTime } from '@/modules/notifications/ui/lib/time'

interface NotificationPanelProps {
  unreadCount: number
  onNavigate: () => void
}

export function NotificationPanel({ unreadCount, onNavigate }: NotificationPanelProps) {
  const { t } = useTranslation('notifications')
  const [unreadOnly, setUnreadOnly] = useState(false)
  const query = useNotifications(unreadOnly, NOTIFICATIONS_PAGE_SIZE)
  const markAll = useMarkAllNotificationsRead()
  const items = query.data?.pages.flatMap((page) => page.data) ?? []

  const markAllRead = () => {
    markAll.mutate(undefined, {
      onError: () => toast({ title: t('panel.markError'), variant: 'destructive' }),
    })
  }

  return (
    <section aria-label={t('panel.title')} className="flex max-h-[min(32rem,80vh)] flex-col">
      <header className="flex items-center justify-between gap-2 border-b px-3 py-2">
        <h2 className="text-sm font-semibold">{t('panel.title')}</h2>
        <Button
          variant="ghost"
          size="sm"
          disabled={unreadCount === 0 || markAll.isPending}
          onClick={markAllRead}
        >
          <CheckCheck aria-hidden="true" />
          {t('panel.markAll')}
        </Button>
      </header>
      <div className="border-b px-3 py-2">
        <SegmentedControl
          aria-label={t('panel.filter')}
          size="sm"
          value={unreadOnly ? 'unread' : 'all'}
          onValueChange={(value) => setUnreadOnly(value === 'unread')}
          options={[
            { value: 'all', label: t('panel.all') },
            { value: 'unread', label: t('panel.unread') },
          ]}
        />
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto">
        {query.isPending ? (
          <div className="grid gap-2 p-3" aria-busy="true">
            <Skeleton className="h-12" />
            <Skeleton className="h-12" />
            <Skeleton className="h-12" />
          </div>
        ) : query.isError ? (
          <div className="grid justify-items-start gap-2 p-3 text-sm">
            <p className="text-destructive">{t('panel.error')}</p>
            <Button variant="outline" size="sm" onClick={() => void query.refetch()}>
              {t('panel.retry')}
            </Button>
          </div>
        ) : items.length === 0 ? (
          <p className="p-6 text-center text-sm text-muted-foreground">
            {t(unreadOnly ? 'panel.emptyUnread' : 'panel.empty')}
          </p>
        ) : (
          <ul className="divide-y">
            {items.map((notification) => (
              <NotificationItem key={notification.id} notification={notification} onNavigate={onNavigate} />
            ))}
          </ul>
        )}
        {query.hasNextPage && (
          <div className="border-t p-2 text-center">
            <Button
              variant="ghost"
              size="sm"
              disabled={query.isFetchingNextPage}
              onClick={() => void query.fetchNextPage()}
            >
              {t('panel.loadMore')}
            </Button>
          </div>
        )}
      </div>
      <footer className="border-t px-3 py-2">
        <Link
          to="/account/notifications"
          onClick={onNavigate}
          className="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
        >
          <Settings className="size-3.5" aria-hidden="true" />
          {t('panel.settings')}
        </Link>
      </footer>
    </section>
  )
}

function NotificationItem({ notification, onNavigate }: { notification: Notification; onNavigate: () => void }) {
  const { t } = useTranslation('notifications')
  const navigate = useNavigate()
  const formatting = useRecordFormatting()
  const markRead = useMarkNotificationRead()
  const headline = notificationHeadline(notification, t)
  const subject = notificationSubject(notification, t)
  const link = notification.link ?? null

  const open = () => {
    if (!notification.read) markRead.mutate(notification.id)
    if (!link) return
    if (isDownloadLink(link)) {
      window.open(link, '_blank', 'noopener')
      return
    }
    onNavigate()
    void navigate(link)
  }

  return (
    <li className={cn('relative', !notification.read && 'bg-accent/40')}>
      <button
        type="button"
        onClick={open}
        className="grid w-full gap-0.5 px-3 py-2.5 text-left text-sm outline-none hover:bg-accent focus-visible:bg-accent"
      >
        <span className="flex items-start gap-2">
          {!notification.read && (
            <span className="mt-1.5 size-2 shrink-0 rounded-full bg-primary" aria-label={t('panel.unreadMark')} />
          )}
          <span className={cn('min-w-0 flex-1', !notification.read && 'font-medium')}>{headline}</span>
        </span>
        {subject && <span className="truncate text-muted-foreground">{subject}</span>}
        {notification.body && <span className="line-clamp-2 text-xs text-muted-foreground">{notification.body}</span>}
        <time dateTime={notification.created_at} className="text-xs text-muted-foreground">
          {relativeTime(notification.created_at, formatting.locale)}
        </time>
      </button>
    </li>
  )
}
