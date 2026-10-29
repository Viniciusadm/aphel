import { useTranslation } from 'react-i18next'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'
import { timeZoneLabel, utcToZonedInput, zonedInputToUtc } from './lib/datetime'
import { ariaProps, type FieldInputBaseProps } from './types'

export interface DateTimeFieldInputProps extends FieldInputBaseProps<string> {
  timeZone: string
  locale: string
}

export function DateTimeFieldInput({ timeZone, locale, ...props }: DateTimeFieldInputProps) {
  const { t } = useTranslation('records-form')
  const local = utcToZonedInput(props.value, timeZone)
  const zone = timeZoneLabel(timeZone, locale, props.value ? new Date(props.value) : undefined)
  return (
    <div className={cn('flex min-w-0 items-center gap-2', props.className)}>
      <Input
        {...ariaProps(props)}
        type="datetime-local"
        className="min-w-0 flex-1"
        value={local}
        onChange={(event) => props.onChange(event.target.value === '' ? null : zonedInputToUtc(event.target.value, timeZone))}
      />
      <span className="shrink-0 text-xs text-muted-foreground" title={timeZone}>
        {t('input.timezone', { zone })}
      </span>
    </div>
  )
}
