import { Alert } from '@/components/ui/alert'

export function FormError({ message }: { message: string | null }) {
  if (!message) return null
  return (
    <Alert role="alert" className="border-destructive/30 bg-destructive/10 text-sm text-destructive">
      {message}
    </Alert>
  )
}
