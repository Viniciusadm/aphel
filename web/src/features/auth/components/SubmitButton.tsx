import { LoaderCircle } from 'lucide-react'
import { Button, type ButtonProps } from '@/components/ui/button'
import { cn } from '@/lib/utils'

interface SubmitButtonProps extends Omit<ButtonProps, 'type'> {
  pending: boolean
  pendingLabel: string
}

export function SubmitButton({ pending, pendingLabel, children, className, disabled, size = 'lg', ...props }: SubmitButtonProps) {
  return (
    <Button
      type="submit"
      size={size}
      className={cn('w-full', className)}
      disabled={pending || disabled}
      aria-busy={pending || undefined}
      {...props}
    >
      {pending && <LoaderCircle className="animate-spin" aria-hidden />}
      {pending ? pendingLabel : children}
    </Button>
  )
}
