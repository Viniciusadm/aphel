export type ToastVariant = 'default' | 'success' | 'destructive'

export interface ToastAction {
  label: string
  altText?: string
  onClick: () => void
}

export interface ToastInput {
  title: string
  description?: string
  variant?: ToastVariant
  duration?: number
  action?: ToastAction
}

export interface ToastRecord extends ToastInput {
  id: number
}

type Listener = () => void

let toasts: ToastRecord[] = []
let nextId = 1
const listeners = new Set<Listener>()

function emit() {
  for (const listener of listeners) listener()
}

export function toast(input: ToastInput): number {
  const id = nextId++
  toasts = [...toasts, { ...input, id }]
  emit()
  return id
}

export function dismissToast(id: number): void {
  toasts = toasts.filter((item) => item.id !== id)
  emit()
}

export function subscribeToToasts(listener: Listener): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

export function getToasts(): ToastRecord[] {
  return toasts
}
