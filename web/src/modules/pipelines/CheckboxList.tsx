import { useId } from 'react'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'

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
            const id = `${baseId}-${item.value}`
            const checked = selected.includes(item.value)
            return (
              <li key={item.value} className="flex min-w-0 items-center gap-2">
                <Checkbox
                  id={id}
                  checked={checked}
                  disabled={disabled}
                  onCheckedChange={(next) =>
                    onChange(
                      next === true ? [...selected, item.value] : selected.filter((value) => value !== item.value),
                    )
                  }
                />
                <Label htmlFor={id} className="min-w-0 truncate font-normal">
                  {item.label}
                </Label>
              </li>
            )
          })}
        </ul>
      )}
    </fieldset>
  )
}
