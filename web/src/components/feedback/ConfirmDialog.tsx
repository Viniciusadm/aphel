import {
  ConfirmDialog as UiConfirmDialog,
  type ConfirmDialogProps as UiConfirmDialogProps,
  type ConfirmDialogVariant,
} from '@/components/ui/confirm-dialog'
import { notifyError } from './notify'

export type { ConfirmDialogVariant }

export interface ConfirmDialogProps extends Omit<UiConfirmDialogProps, 'onError'> {
  notifyOnError?: boolean
}

export function ConfirmDialog({ notifyOnError = true, ...props }: ConfirmDialogProps) {
  return <UiConfirmDialog {...props} onError={notifyOnError ? (error) => void notifyError(error) : undefined} />
}
