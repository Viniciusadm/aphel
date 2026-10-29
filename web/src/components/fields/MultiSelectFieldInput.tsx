import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import type { FieldOption } from '@/features/records/types'
import { OptionLabel } from './OptionLabel'
import { SearchablePicker } from './SearchablePicker'
import type { FieldInputBaseProps } from './types'

export interface MultiSelectFieldInputProps extends FieldInputBaseProps<string[]> {
  options: FieldOption[]
}

function matches(option: FieldOption, search: string) {
  return option.label.toLocaleLowerCase().includes(search.trim().toLocaleLowerCase())
}

export function MultiSelectFieldInput({ options, ...props }: MultiSelectFieldInputProps) {
  const { t } = useTranslation('records-form')
  const [open, setOpen] = useState(false)
  const [search, setSearch] = useState('')
  const selected = props.value ?? []
  const byValue = new Map(options.map((option) => [option.value, option]))
  const visible = options.filter((option) => matches(option, search))

  const toggle = (value: string) => {
    const next = selected.includes(value) ? selected.filter((item) => item !== value) : [...selected, value]
    props.onChange(next.length > 0 ? next : null)
  }

  return (
    <SearchablePicker
      id={props.id}
      multiple
      open={open}
      onOpenChange={setOpen}
      search={search}
      onSearchChange={setSearch}
      selected={selected}
      onSelect={toggle}
      options={visible.map((option) => ({
        value: option.value,
        label: option.label,
        content: <OptionLabel option={option} />,
      }))}
      disabled={props.disabled}
      invalid={props.invalid}
      autoFocus={props.autoFocus}
      className={props.className}
      onBlur={props.onBlur}
      aria-describedby={props['aria-describedby']}
      aria-labelledby={props['aria-labelledby']}
      aria-label={props['aria-label']}
    >
      {selected.length === 0 ? (
        <span className="text-muted-foreground">{t('input.placeholder')}</span>
      ) : (
        selected.map((value) => {
          const option = byValue.get(value)
          return (
            <span key={value} className="inline-flex max-w-full items-center rounded-md border px-1.5 py-0.5 text-xs font-medium">
              {option ? <OptionLabel option={option} /> : value}
            </span>
          )
        })
      )}
    </SearchablePicker>
  )
}
