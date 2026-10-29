import { Bell, BellOff } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Avatar } from '@/components/ui/avatar'
import { Button } from '@/components/ui/button'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { toast } from '@/components/ui/toast-store'
import { errorMessage } from '@/components/feedback'
import { useFollowToggle, useRecordFollowers } from './api'

export interface FollowRecordButtonProps {
  recordType: string
  recordId: string
}

export function FollowRecordButton({ recordType, recordId }: FollowRecordButtonProps) {
  const { t } = useTranslation('followers')
  const followers = useRecordFollowers(recordType, recordId)
  const toggle = useFollowToggle(recordType, recordId)

  if (followers.isError) return null

  const following = followers.data?.following ?? false
  const total = followers.data?.total ?? 0
  const list = followers.data?.followers ?? []

  return (
    <div className="flex items-center">
      <Button
        variant={following ? 'secondary' : 'outline'}
        size="sm"
        className="rounded-r-none"
        aria-pressed={following}
        disabled={!followers.data || toggle.isPending}
        onClick={() =>
          toggle.mutate(!following, {
            onError: (error) =>
              toast({ title: t('toast.errorTitle'), description: errorMessage(error), variant: 'destructive' }),
          })
        }
      >
        {following ? <BellOff aria-hidden /> : <Bell aria-hidden />}
        {following ? t('button.unfollow') : t('button.follow')}
      </Button>
      <Popover>
        <PopoverTrigger asChild>
          <Button
            variant="outline"
            size="sm"
            className="-ml-px rounded-l-none px-2.5 tabular-nums"
            aria-label={t('button.count', { count: total })}
            disabled={!followers.data}
          >
            {total}
          </Button>
        </PopoverTrigger>
        <PopoverContent align="end" className="grid gap-2">
          <p className="text-sm font-medium">{t('list.title')}</p>
          {list.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t('list.empty')}</p>
          ) : (
            <ul className="grid max-h-64 gap-2 overflow-y-auto">
              {list.map((follower) => (
                <li key={follower.user.id} className="flex items-center gap-2 text-sm">
                  <Avatar name={follower.user.display_name} />
                  <span className="min-w-0 flex-1 truncate">{follower.user.display_name}</span>
                  {follower.origin === 'owner' && (
                    <span className="text-xs text-muted-foreground">{t('list.owner')}</span>
                  )}
                </li>
              ))}
            </ul>
          )}
        </PopoverContent>
      </Popover>
    </div>
  )
}
