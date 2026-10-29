import { useTranslation } from 'react-i18next'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import type { FieldOption } from '@/features/records/types'
import { OptionLabel } from './OptionLabel'
import type { FieldInputBaseProps } from './types'

const EMPTY_VALUE = '__aphel_empty__'

export interface SelectFieldInputProps extends FieldInputBaseProps<string> {
  options: FieldOption[]
  clearable?: boolean
}

export function SelectFieldInput({ options, clearable = true, ...props }: SelectFieldInputProps) {
  const { t } = useTranslation('records-form')
  return (
    <Select
      value={props.value ?? ''}
      disabled={props.disabled}
      onValueChange={(next) => props.onChange(next === EMPTY_VALUE ? null : next)}
      onOpenChange={(open) => {
        if (!open) props.onBlur?.()
      }}
    >
      <SelectTrigger
        id={props.id}
        autoFocus={props.autoFocus}
        className={props.className}
        aria-invalid={props.invalid ? true : undefined}
        aria-describedby={props['aria-describedby']}
        aria-labelledby={props['aria-labelledby']}
        aria-label={props['aria-label']}
      >
        <SelectValue placeholder={t('input.placeholder')} />
      </SelectTrigger>
      <SelectContent className="max-w-[calc(100vw-2rem)]">
        {clearable && (
          <SelectItem value={EMPTY_VALUE}>
            <span className="text-muted-foreground">{t('input.none')}</span>
          </SelectItem>
        )}
        {options.map((option) => (
          <SelectItem key={option.value} value={option.value}>
            <OptionLabel option={option} />
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
