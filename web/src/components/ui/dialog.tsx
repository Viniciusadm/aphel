import { X } from 'lucide-react'
import { Dialog as DialogPrimitive } from 'radix-ui'
import type { ComponentProps } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'

export const Dialog = DialogPrimitive.Root
export const DialogTrigger = DialogPrimitive.Trigger
export const DialogPortal = DialogPrimitive.Portal
export const DialogClose = DialogPrimitive.Close

export function DialogOverlay({ className, ...props }: ComponentProps<typeof DialogPrimitive.Overlay>) {
  return (
    <DialogPrimitive.Overlay
      data-slot="dialog-overlay"
      className={cn('fixed inset-0 z-50 bg-black/50', className)}
      {...props}
    />
  )
}

type DialogSide = 'center' | 'left' | 'right'

const sideClasses: Record<DialogSide, string> = {
  center:
    'top-1/2 left-1/2 max-h-[calc(100dvh-2rem)] w-full max-w-[calc(100%-2rem)] -translate-x-1/2 -translate-y-1/2 rounded-lg border [--dialog-gap:--spacing(4)] [--dialog-padding:--spacing(6)] sm:max-w-lg',
  left: 'inset-y-0 left-0 h-full w-72 max-w-[85vw] border-r [--dialog-gap:0px] [--dialog-padding:0px]',
  right: 'inset-y-0 right-0 h-full w-72 max-w-[85vw] border-l [--dialog-gap:0px] [--dialog-padding:0px]',
}

const bodySideClasses: Record<DialogSide, string> = {
  center: '*:shrink-0',
  left: '',
  right: '',
}

export interface DialogContentProps extends ComponentProps<typeof DialogPrimitive.Content> {
  side?: DialogSide
  showCloseButton?: boolean
}

export function DialogContent({
  className,
  children,
  side = 'center',
  showCloseButton = true,
  ...props
}: DialogContentProps) {
  const { t } = useTranslation()
  return (
    <DialogPortal>
      <DialogOverlay />
      <DialogPrimitive.Content
        data-slot="dialog-content"
        className={cn(
          'fixed z-50 flex flex-col overflow-hidden bg-background text-foreground shadow-lg outline-none',
          sideClasses[side],
          className,
        )}
        {...props}
      >
        <div
          data-slot="dialog-body"
          className={cn(
            'flex min-h-0 flex-auto flex-col gap-(--dialog-gap) overflow-y-auto p-(--dialog-padding) has-[[data-slot=dialog-footer]]:pb-0',
            bodySideClasses[side],
          )}
        >
          {children}
        </div>
        {showCloseButton && (
          <DialogPrimitive.Close
            className="absolute top-4 right-4 z-20 rounded-sm bg-inherit text-foreground/70 transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
            aria-label={t('actions.close')}
          >
            <X className="size-4" />
          </DialogPrimitive.Close>
        )}
      </DialogPrimitive.Content>
    </DialogPortal>
  )
}

export function DialogHeader({ className, ...props }: ComponentProps<'div'>) {
  return <div data-slot="dialog-header" className={cn('flex flex-col gap-2 text-left', className)} {...props} />
}

export function DialogFooter({ className, ...props }: ComponentProps<'div'>) {
  return (
    <div
      data-slot="dialog-footer"
      className={cn(
        'sticky bottom-0 z-10 -mx-(--dialog-padding) flex flex-col-reverse gap-2 border-t bg-background px-(--dialog-padding) pt-4 pb-(--dialog-padding) sm:flex-row sm:justify-end',
        className,
      )}
      {...props}
    />
  )
}

export function DialogTitle({ className, ...props }: ComponentProps<typeof DialogPrimitive.Title>) {
  return (
    <DialogPrimitive.Title
      data-slot="dialog-title"
      className={cn('text-lg leading-none font-semibold', className)}
      {...props}
    />
  )
}

export function DialogDescription({ className, ...props }: ComponentProps<typeof DialogPrimitive.Description>) {
  return (
    <DialogPrimitive.Description
      data-slot="dialog-description"
      className={cn('text-sm text-muted-foreground', className)}
      {...props}
    />
  )
}
