import { useId } from 'react'
import { useUiLabels } from './labels-context'
import { NativeSelect } from './native-select'

export interface PageSizeSelectProps {
  value: number
  onChange: (value: number) => void
  options?: readonly number[]
}

export function PageSizeSelect({ value, onChange, options = [25, 50, 100] }: PageSizeSelectProps) {
  const labels = useUiLabels()
  const id = useId()
  const sizes = [...new Set([value, ...options])].sort((left, right) => left - right)
  return <label htmlFor={id} className="grid gap-2 text-sm">
    <span>{labels.paginationPerPage}</span>
    <NativeSelect id={id} className="w-full" value={String(value)} onValueChange={(next) => onChange(Number(next))}>
      {sizes.map((size) => <option key={size} value={size}>{size}</option>)}
    </NativeSelect>
  </label>
}
