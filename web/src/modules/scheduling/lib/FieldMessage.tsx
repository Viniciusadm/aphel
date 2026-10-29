import { useSchedulingText } from '../text'

interface FieldMessageProps {
  id?: string
  message?: string
}

export function FieldMessage({ id, message }: FieldMessageProps) {
  const text = useSchedulingText()
  if (!message) return null
  return (
    <p id={id} className="text-sm text-destructive">
      {text(message)}
    </p>
  )
}
