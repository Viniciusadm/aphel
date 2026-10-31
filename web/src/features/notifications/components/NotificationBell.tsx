import { Badge } from '@/components/ui/badge'
import { Bell } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { useUnreadNotificationCount } from '../api'
import { NotificationPanel } from './NotificationPanel'

export const MAX_BADGE_COUNT = 99

export function NotificationBell() {
  const { t } = useTranslation('notifications')
  const [open, setOpen] = useState(false)
  const { data: count = 0 } = useUnreadNotificationCount()
  const label = count > 0 ? t('bell.labelWithCount', { count }) : t('bell.label')

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button variant="ghost" size="icon" className="relative" aria-label={label}>
          <Bell />
          {count > 0 && (
            <Badge
              count
              size="sm"
              variant="destructive"
              aria-hidden="true"
              className="absolute -top-0.5 -right-0.5"
            >
              {count > MAX_BADGE_COUNT ? `${MAX_BADGE_COUNT}+` : count}
            </Badge>
          )}
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-96 p-0">
        <NotificationPanel unreadCount={count} onNavigate={() => setOpen(false)} />
      </PopoverContent>
    </Popover>
  )
}
