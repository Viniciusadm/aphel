import { CalendarClock, CalendarPlus, Check, Handshake, MoreHorizontal, UserRound, UserRoundPen } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import type { InboxGroupKey, InboxItem } from '@/modules/tasks/api'
import { ActivityTypeIcon } from '@/modules/tasks/components/ActivityTypeIcon'
import { activityResource } from '@/modules/tasks/resource'
import { formatCurrency, formatDateTime, getRecordResource, useRecordFormatting } from '@/sdk'
import { cn } from '@/lib/utils'

export interface InboxItemRowProps {
  item: InboxItem
  group: InboxGroupKey
  showAmounts: boolean
  pending: boolean
  onComplete: (item: InboxItem) => void
  onPostpone: (item: InboxItem, days: number) => void
  onReassign: (item: InboxItem) => void
}

export function InboxItemRow({
  item,
  group,
  showAmounts,
  pending,
  onComplete,
  onPostpone,
  onReassign,
}: InboxItemRowProps) {
  const { t } = useTranslation('tasks', { keyPrefix: 'inbox' })
  const formatting = useRecordFormatting()
  const detail = activityResource.detailPath?.(item.id)
  const recordHref = item.record ? getRecordResource(item.record.record_type)?.detailPath?.(item.record.record_id) : undefined
  const deal = item.linked_deal
  const dealHref = deal ? getRecordResource('deal')?.detailPath?.(deal.deal_id) : undefined
  const amount = showAmounts && deal?.amount ? formatCurrency(deal.amount, formatting, deal.currency) : ''
  const showDealSeparately = deal && !(item.record?.record_type === 'deals.deal' && item.record.record_id === deal.deal_id)

  return (
    <li
      data-activity-id={item.id}
      aria-busy={pending || undefined}
      className={cn(
        'flex items-start gap-3 rounded-md border bg-card px-3 py-2.5 text-card-foreground',
        pending && 'opacity-60',
      )}
    >
      <Checkbox
        className="mt-1.5"
        checked={false}
        disabled={pending}
        onCheckedChange={(checked) => checked === true && onComplete(item)}
        aria-label={t('actions.completeLabel', { title: item.title })}
      />
      <ActivityTypeIcon icon={item.activity_type.icon} color={item.activity_type.color} />
      <div className="grid min-w-0 flex-1 gap-1">
        <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
          {detail ? (
            <Link to={detail} className="min-w-0 truncate text-sm font-medium hover:underline">
              {item.title}
            </Link>
          ) : (
            <span className="min-w-0 truncate text-sm font-medium">{item.title}</span>
          )}
          {(item.priority === 'high' || item.priority === 'urgent') && (
            <Badge variant={item.priority === 'urgent' ? 'destructive' : 'warning'}>{t(`priority.${item.priority}`)}</Badge>
          )}
          {amount && (
            <Badge variant="outline" className="font-medium">
              {amount}
            </Badge>
          )}
        </div>
        <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
          <span>{item.activity_type.name}</span>
          {item.due_at && (
            <span
              className={cn(
                'inline-flex items-center gap-1',
                group === 'overdue' && 'font-medium text-destructive',
                group === 'today' && 'font-medium text-foreground',
              )}
            >
              <CalendarClock className="size-3" aria-hidden="true" />
              {t(group === 'overdue' ? 'item.overdue' : 'item.due', { date: formatDateTime(item.due_at, formatting) })}
            </span>
          )}
          {item.owner && (
            <span className="inline-flex items-center gap-1">
              <UserRound className="size-3" aria-hidden="true" />
              {item.owner.display_name}
            </span>
          )}
          {item.record &&
            (recordHref ? (
              <Link to={recordHref} className="max-w-full truncate text-primary underline-offset-4 hover:underline">
                {item.record.title || t('item.relatedRecord')}
              </Link>
            ) : (
              <span className="truncate">{item.record.title || t('item.relatedRecord')}</span>
            ))}
          {showDealSeparately && deal && (
            <span className="inline-flex min-w-0 items-center gap-1">
              <Handshake className="size-3 shrink-0" aria-hidden="true" />
              {dealHref ? (
                <Link to={dealHref} className="truncate text-primary underline-offset-4 hover:underline">
                  {deal.title}
                </Link>
              ) : (
                <span className="truncate">{deal.title}</span>
              )}
            </span>
          )}
        </div>
      </div>
      <div className="flex shrink-0 items-center gap-1">
        <div className="hidden items-center gap-1 md:flex">
          <Button variant="ghost" size="sm" disabled={pending} onClick={() => onPostpone(item, 1)}>
            {t('actions.postponeDay')}
          </Button>
          <Button variant="ghost" size="sm" disabled={pending} onClick={() => onPostpone(item, 7)}>
            {t('actions.postponeWeek')}
          </Button>
        </div>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="icon" disabled={pending} aria-label={t('actions.menu', { title: item.title })}>
              <MoreHorizontal aria-hidden="true" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onSelect={() => onComplete(item)}>
              <Check aria-hidden="true" />
              {t('actions.complete')}
            </DropdownMenuItem>
            <DropdownMenuItem onSelect={() => onPostpone(item, 1)}>
              <CalendarPlus aria-hidden="true" />
              {t('actions.postponeDay')}
            </DropdownMenuItem>
            <DropdownMenuItem onSelect={() => onPostpone(item, 7)}>
              <CalendarPlus aria-hidden="true" />
              {t('actions.postponeWeek')}
            </DropdownMenuItem>
            <DropdownMenuItem onSelect={() => onReassign(item)}>
              <UserRoundPen aria-hidden="true" />
              {t('actions.reassign')}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </li>
  )
}
