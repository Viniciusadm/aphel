import { useTranslation } from 'react-i18next'
import type { FieldOption } from '@/features/records/types'

export function OptionLabel({ option }: { option: FieldOption }) {
  const { t } = useTranslation('records-form')
  return (
    <span className="inline-flex min-w-0 items-center gap-2">
      <span
        aria-hidden="true"
        className="size-2 shrink-0 rounded-full bg-muted-foreground"
        style={option.color ? { backgroundColor: option.color } : undefined}
      />
      <span className="truncate">
        {option.is_active ? option.label : t('input.inactiveOption', { label: option.label })}
      </span>
    </span>
  )
}
