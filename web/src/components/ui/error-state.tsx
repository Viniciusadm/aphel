import { CircleAlert } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from './button'
import { useUiLabels } from './labels-context'

export interface ErrorStateProps {
  title?: string
  description?: string
  onRetry?: () => void
  className?: string
}

export function ErrorState({ title, description, onRetry, className }: ErrorStateProps) {
  const labels = useUiLabels()
  return (
    <div
      role="alert"
      data-slot="error-state"
      className={cn(
        'flex flex-col items-center justify-center gap-3 rounded-lg border border-destructive/30 bg-destructive/5 p-8 text-center',
        className,
      )}
    >
      <CircleAlert className="size-8 text-destructive" aria-hidden />
      <div className="grid gap-1">
        <p className="text-base font-medium">{title ?? labels.errorTitle}</p>
        <p className="max-w-sm text-sm text-muted-foreground">{description ?? labels.errorDescription}</p>
      </div>
      {onRetry && (
        <Button variant="outline" size="sm" onClick={onRetry}>
          {labels.retry}
        </Button>
      )}
    </div>
  )
}
