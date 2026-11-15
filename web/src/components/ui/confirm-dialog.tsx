import { useId, useState, type ReactNode } from 'react'
import { Button } from './button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle, type DialogSize } from './dialog'
import { Input } from './input'
import { Label } from './label'
import { useUiLabels } from './labels-context'

export type ConfirmDialogVariant = 'default' | 'destructive'

export interface ConfirmDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  title: string
  description?: ReactNode
  children?: ReactNode
  confirmLabel?: string
  cancelLabel?: string
  variant?: ConfirmDialogVariant
  confirmationText?: string
  confirmDisabled?: boolean
  size?: DialogSize
  pending?: boolean
  onError?: (error: unknown) => void
  onConfirm: () => void | Promise<void>
}

interface ConfirmDialogBodyProps
  extends Omit<ConfirmDialogProps, 'open' | 'pending' | 'onError' | 'onConfirm'> {
  busy: boolean
  onSubmit: () => void
}

function ConfirmDialogBody({
  title,
  description,
  children,
  confirmLabel,
  cancelLabel,
  variant,
  confirmationText,
  confirmDisabled = false,
  busy,
  onOpenChange,
  onSubmit,
}: ConfirmDialogBodyProps) {
  const labels = useUiLabels()
  const inputId = useId()
  const [typed, setTyped] = useState('')
  const destructive = variant === 'destructive'
  const requiresTyping = destructive && Boolean(confirmationText)
  const typedMatches = !requiresTyping || typed.trim() === confirmationText?.trim()

  const submit = () => {
    if (!busy && typedMatches && !confirmDisabled) onSubmit()
  }

  return (
    <>
      <DialogHeader>
        <DialogTitle>{title}</DialogTitle>
        {description && <DialogDescription>{description}</DialogDescription>}
      </DialogHeader>
      {children}
      {requiresTyping && (
        <form
          className="grid gap-2"
          onSubmit={(event) => {
            event.preventDefault()
            submit()
          }}
        >
          <Label htmlFor={inputId} className="block text-sm leading-normal font-normal">
            {labels.confirmTypePrompt(confirmationText ?? '')}
          </Label>
          <Input
            placeholder={confirmationText}
            id={inputId}
            value={typed}
            autoComplete="off"
            autoCapitalize="off"
            spellCheck={false}
            disabled={busy}
            aria-label={labels.confirmTypeLabel}
            onChange={(event) => setTyped(event.target.value)}
          />
        </form>
      )}
      <DialogFooter>
        <Button variant="outline" onClick={() => onOpenChange(false)} disabled={busy}>
          {cancelLabel ?? labels.confirmCancel}
        </Button>
        <Button
          variant={destructive ? 'destructive' : 'default'}
          onClick={submit}
          disabled={busy || !typedMatches || confirmDisabled}
          aria-busy={busy}
        >
          {busy
            ? labels.confirmPending
            : (confirmLabel ?? (destructive ? labels.confirmDelete : labels.confirmConfirm))}
        </Button>
      </DialogFooter>
    </>
  )
}

export function ConfirmDialog({
  open,
  onOpenChange,
  pending = false,
  onError,
  onConfirm,
  variant = 'default',
  size,
  ...body
}: ConfirmDialogProps) {
  const [submitting, setSubmitting] = useState(false)
  const busy = pending || submitting

  const changeOpen = (next: boolean) => {
    if (busy && !next) return
    onOpenChange(next)
  }

  const confirm = async () => {
    setSubmitting(true)
    try {
      await onConfirm()
      onOpenChange(false)
    } catch (error) {
      onError?.(error)
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={changeOpen}>
      <DialogContent size={size} showCloseButton={!busy} aria-busy={busy}>
        <ConfirmDialogBody
          {...body}
          variant={variant}
          busy={busy}
          onOpenChange={changeOpen}
          onSubmit={() => void confirm()}
        />
      </DialogContent>
    </Dialog>
  )
}
