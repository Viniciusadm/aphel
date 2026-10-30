import { Check } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'

export interface ProgressStep {
  key: string
  label: string
}

export function SetupProgress({ steps, current }: { steps: ProgressStep[]; current: number }) {
  const { t } = useTranslation('setup')
  const active = steps[current]
  return (
    <nav aria-label={t('progress.label')} className="grid gap-2">
      <div className="grid gap-2 sm:hidden">
        <p className="text-xs font-medium text-muted-foreground">
          {t('progress.step', { current: current + 1, total: steps.length })}
          {active && ` · ${active.label}`}
        </p>
        <div
          role="progressbar"
          aria-label={t('progress.label')}
          aria-valuemin={1}
          aria-valuemax={steps.length}
          aria-valuenow={current + 1}
          className="h-1.5 w-full overflow-hidden rounded-full bg-muted"
        >
          <div
            className="h-full rounded-full bg-primary-solid transition-[width] duration-300"
            style={{ width: `${((current + 1) / steps.length) * 100}%` }}
          />
        </div>
      </div>
      <ol className="hidden items-start sm:flex">
        {steps.map((step, index) => {
          const done = index < current
          const isCurrent = index === current
          return (
            <li key={step.key} className="relative flex flex-1 flex-col items-center gap-1.5 text-center">
              {index > 0 && (
                <span
                  aria-hidden
                  className={cn(
                    'absolute top-3.5 right-1/2 h-px w-full -translate-y-1/2',
                    index <= current ? 'bg-primary-solid' : 'bg-border',
                  )}
                />
              )}
              <span
                aria-hidden
                className={cn(
                  'relative z-10 flex size-7 items-center justify-center rounded-full border text-xs font-semibold transition-colors',
                  done && 'border-primary-solid bg-primary-solid text-primary-foreground',
                  isCurrent && 'border-primary-solid bg-background text-primary ring-4 ring-primary/15',
                  !done && !isCurrent && 'bg-background text-muted-foreground',
                )}
              >
                {done ? <Check className="size-3.5" /> : index + 1}
              </span>
              <span
                aria-current={isCurrent ? 'step' : undefined}
                className={cn('px-1 text-xs', isCurrent ? 'font-medium text-foreground' : 'text-muted-foreground')}
              >
                {step.label}
                <span className="sr-only">
                  {done ? ` (${t('progress.done')})` : isCurrent ? ` (${t('progress.current')})` : ''}
                </span>
              </span>
            </li>
          )
        })}
      </ol>
    </nav>
  )
}
