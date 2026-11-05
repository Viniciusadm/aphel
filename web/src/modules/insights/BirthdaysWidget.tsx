import { Cake, Settings } from 'lucide-react'
import { Link } from 'react-router'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { useRecordFormatting } from '@/sdk'
import { cn } from '@/lib/utils'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import type { BirthdaysData } from './types'

export function BirthdaysWidget({ data }: { data: BirthdaysData }) {
  const t = useModuleTranslation('insights')
  const formatting = useRecordFormatting()
  if (data.status !== 'ok') {
    return (
      <EmptyState
        icon={Cake}
        className="p-4"
        title={t(data.status === 'not_configured' ? 'birthdays.notConfigured' : 'birthdays.fieldUnavailable')}
        description={t(data.status === 'not_configured' ? 'birthdays.notConfiguredHelp' : 'birthdays.fieldUnavailableHelp')}
        action={
          <Button asChild variant="outline" size="sm">
            <Link to="/integrations/modules">
              <Settings aria-hidden />
              {t('birthdays.configure')}
            </Link>
          </Button>
        }
      />
    )
  }
  const month = new Intl.DateTimeFormat(formatting.locale, { month: 'long', timeZone: 'UTC' }).format(
    new Date(Date.UTC(2000, data.month - 1, 1)),
  )
  const remaining = data.total - data.items.length
  return (
    <div className="grid gap-2">
      <p className="text-xs text-muted-foreground">
        {t('birthdays.summary', { count: data.total, month, field: data.field_label ?? '' })}
      </p>
      <ul className="grid gap-1.5" aria-label={t('birthdays.listLabel', { month })}>
        {data.items.map((item) => (
          <li
            key={item.id}
            className={cn('flex min-w-0 items-center gap-3 rounded-md border px-3 py-2 text-sm', item.is_today && 'border-primary bg-primary/5')}
          >
            <span
              className={cn(
                'flex size-8 shrink-0 items-center justify-center rounded-full text-xs font-semibold tabular-nums',
                item.is_today ? 'bg-primary-solid text-primary-foreground' : 'bg-muted text-muted-foreground',
              )}
              aria-label={t('birthdays.day', { day: item.day })}
            >
              {item.day}
            </span>
            <Link to={`/people/${encodeURIComponent(item.id)}`} className="min-w-0 truncate font-medium hover:underline">
              {item.name}
            </Link>
            {item.is_today && (
              <span className="ml-auto inline-flex shrink-0 items-center gap-1 text-xs font-medium text-primary">
                <Cake className="size-3.5" aria-hidden />
                {t('birthdays.today')}
              </span>
            )}
          </li>
        ))}
      </ul>
      {remaining > 0 && <p className="text-xs text-muted-foreground">{t('birthdays.more', { count: remaining })}</p>}
      {data.truncated && <p className="text-xs text-muted-foreground">{t('birthdays.truncated')}</p>}
    </div>
  )
}
