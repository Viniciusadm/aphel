import { Rows3, Rows4 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import type { TableDensity } from '../types'

export interface DensityToggleProps {
  value: TableDensity
  onChange: (density: TableDensity) => void
  className?: string
}

const OPTIONS = [
  { value: 'comfortable', icon: Rows3 },
  { value: 'compact', icon: Rows4 },
] as const

export function DensityToggle({ value, onChange, className }: DensityToggleProps) {
  const { t } = useTranslation('records-table')
  return (
    <div
      role="radiogroup"
      aria-label={t('density.label')}
      className={cn('inline-flex h-8 items-center rounded-md border p-0.5', className)}
    >
      {OPTIONS.map((option) => {
        const Icon = option.icon
        const checked = option.value === value
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={checked}
            aria-label={t(`density.${option.value}`)}
            title={t(`density.${option.value}`)}
            className={cn(
              'flex h-full items-center justify-center rounded-sm px-2 text-muted-foreground outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring',
              checked && 'bg-accent text-accent-foreground',
            )}
            onClick={() => onChange(option.value)}
          >
            <Icon className="size-4" aria-hidden />
          </button>
        )
      })}
    </div>
  )
}
