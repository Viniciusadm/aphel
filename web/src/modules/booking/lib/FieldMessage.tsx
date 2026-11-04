import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useBookingText } from '../text'

interface FieldMessageProps {
  id?: string
  message?: string
}

export function FieldMessage({ id, message }: FieldMessageProps) {
  const text = useBookingText()
  if (!message) return null
  return (
    <SharedFieldError id={id}>
      {text(message)}
    </SharedFieldError>
  )
}
