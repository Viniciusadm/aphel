import { useId, useState, type ReactNode } from 'react'
import { Trans, useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { notifyError } from './notify'

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
  pending?: boolean
  notifyOnError?: boolean
  onConfirm: () => void | Promise<void>
}

interface ConfirmDialogBodyProps extends Omit<ConfirmDialogProps, 'open' | 'pending' | 'notifyOnError' | 'onConfirm'> {
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
  busy,
  onOpenChange,
  onSubmit,
}: ConfirmDialogBodyProps) {
  const { t } = useTranslation('records-detail')
  const inputId = useId()
  const [typed, setTyped] = useState('')
  const destructive = variant === 'destructive'
  const requiresTyping = destructive && Boolean(confirmationText)
  const typedMatches = !requiresTyping || typed.trim() === confirmationText?.trim()

  const submit = () => {
    if (!busy && typedMatches) onSubmit()
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
            <Trans
              t={t}
              i18nKey="confirm.typeToConfirm"
              values={{ name: confirmationText }}
              components={{ strong: <strong className="font-semibold break-all" /> }}
            />
          </Label>
          <Input
            id={inputId}
            value={typed}
            autoComplete="off"
            autoCapitalize="off"
            spellCheck={false}
            disabled={busy}
            aria-label={t('confirm.typeToConfirmLabel')}
            onChange={(event) => setTyped(event.target.value)}
          />
        </form>
      )}
      <DialogFooter>
        <Button variant="outline" onClick={() => onOpenChange(false)} disabled={busy}>
          {cancelLabel ?? t('confirm.cancel')}
        </Button>
        <Button
          variant={destructive ? 'destructive' : 'default'}
          onClick={submit}
          disabled={busy || !typedMatches}
          aria-busy={busy}
        >
          {busy ? t('confirm.pending') : (confirmLabel ?? (destructive ? t('confirm.delete') : t('confirm.confirm')))}
        </Button>
      </DialogFooter>
    </>
  )
}

export function ConfirmDialog({
  open,
  onOpenChange,
  pending = false,
  notifyOnError = true,
  onConfirm,
  variant = 'default',
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
      if (notifyOnError) notifyError(error)
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={changeOpen}>
      <DialogContent showCloseButton={!busy} aria-busy={busy}>
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
