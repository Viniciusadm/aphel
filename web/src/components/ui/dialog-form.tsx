import type { ComponentProps, ReactNode } from 'react'
import { cn } from '@/lib/utils'
import { Alert } from './alert'
import { Button, type ButtonProps } from './button'
import { DialogDescription, DialogFooter, DialogHeader, DialogTitle } from './dialog'

export interface DialogFormProps extends Omit<ComponentProps<'form'>, 'title'> {
  title: ReactNode
  description?: ReactNode
  error?: ReactNode
  pending?: boolean
  submitDisabled?: boolean
  submitVariant?: ButtonProps['variant']
  submitLabel: ReactNode
  cancelLabel: ReactNode
  onCancel: () => void
}

export function DialogForm({ title, description, error, pending, submitDisabled, submitLabel, submitVariant, cancelLabel, onCancel, children, className, onSubmit, ...props }: DialogFormProps) {
  return (
    <form {...props} aria-busy={pending || undefined} className={cn('grid gap-4', className)} onSubmit={(event) => {
      event.preventDefault()
      if (!pending && !submitDisabled) onSubmit?.(event)
    }}>
      <DialogHeader>
        <DialogTitle>{title}</DialogTitle>
        {description && <DialogDescription>{description}</DialogDescription>}
      </DialogHeader>
      {error && <Alert>{error}</Alert>}
      {children}
      <DialogFooter>
        <Button type="button" variant="outline" disabled={pending} onClick={onCancel}>{cancelLabel}</Button>
        <Button type="submit" variant={submitVariant} aria-busy={pending || undefined} disabled={pending || submitDisabled}>{submitLabel}</Button>
      </DialogFooter>
    </form>
  )
}
