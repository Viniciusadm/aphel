import { useId } from 'react'
import { ToggleField } from '@/components/ui/toggle-field'

export interface CheckboxListItem {
  value: string
  label: string
}

export interface CheckboxListProps {
  legend: string
  description?: string
  items: CheckboxListItem[]
  selected: string[]
  emptyText?: string
  disabled?: boolean
  onChange: (selected: string[]) => void
}

export function CheckboxList({
  legend,
  description,
  items,
  selected,
  emptyText,
  disabled,
  onChange,
}: CheckboxListProps) {
  const baseId = useId()
  const descriptionId = `${baseId}-description`

  return (
    <fieldset className="grid gap-2" aria-describedby={description ? descriptionId : undefined}>
      <legend className="text-sm font-medium">{legend}</legend>
      {description && (
        <p id={descriptionId} className="text-xs text-muted-foreground">
          {description}
        </p>
      )}
      {items.length === 0 && emptyText ? (
        <p className="text-sm text-muted-foreground">{emptyText}</p>
      ) : (
        <ul className="grid max-h-48 gap-2 overflow-y-auto sm:grid-cols-2">
          {items.map((item) => {
            const checked = selected.includes(item.value)
            return (
              <li key={item.value} className="min-w-0">
                <ToggleField
                  label={item.label}
                  checked={checked}
                  disabled={disabled}
                  onCheckedChange={(next) =>
                    onChange(next ? [...selected, item.value] : selected.filter((value) => value !== item.value))
                  }
                />
              </li>
            )
          })}
        </ul>
      )}
    </fieldset>
  )
}
