import { X } from 'lucide-react'
import { Toast as ToastPrimitive } from 'radix-ui'
import { useSyncExternalStore } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { dismissToast, getToasts, subscribeToToasts, type ToastVariant } from './toast-store'

const variantClasses: Record<ToastVariant, string> = {
  default: 'border bg-popover text-popover-foreground',
  success: 'border-success/40 bg-popover text-popover-foreground [&_[data-slot=toast-title]]:text-success',
  destructive: 'border-destructive-solid bg-destructive-solid text-destructive-foreground',
}

export function Toaster() {
  const { t } = useTranslation()
  const toasts = useSyncExternalStore(subscribeToToasts, getToasts, getToasts)

  return (
    <ToastPrimitive.Provider swipeDirection="right" label={t('toast.label')}>
      {toasts.map((item) => (
        <ToastPrimitive.Root
          key={item.id}
          duration={item.duration ?? 5000}
          onOpenChange={(open) => {
            if (!open) dismissToast(item.id)
          }}
          className={cn(
            'relative flex w-full items-start gap-3 rounded-md p-4 pr-8 shadow-lg',
            variantClasses[item.variant ?? 'default'],
          )}
        >
          <div className="grid gap-1">
            <ToastPrimitive.Title data-slot="toast-title" className="text-sm font-semibold">
              {item.title}
            </ToastPrimitive.Title>
            {item.description && (
              <ToastPrimitive.Description className="text-sm opacity-90">{item.description}</ToastPrimitive.Description>
            )}
            {item.action && (
              <ToastPrimitive.Action
                altText={item.action.altText ?? item.action.label}
                onClick={item.action.onClick}
                className="mt-1 inline-flex h-8 w-fit items-center rounded-md border border-current/30 px-3 text-sm font-medium hover:bg-foreground/5 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
              >
                {item.action.label}
              </ToastPrimitive.Action>
            )}
          </div>
          <ToastPrimitive.Close
            aria-label={t('actions.close')}
            className="absolute top-2 right-2 rounded-sm p-1 opacity-70 hover:opacity-100 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
          >
            <X className="size-4" />
          </ToastPrimitive.Close>
        </ToastPrimitive.Root>
      ))}
      <ToastPrimitive.Viewport
        label={t('toast.viewport')}
        className="fixed right-0 bottom-0 z-[100] m-0 flex w-full max-w-sm list-none flex-col gap-2 p-4 outline-none"
      />
    </ToastPrimitive.Provider>
  )
}
