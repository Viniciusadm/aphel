import { Eye, EyeOff } from 'lucide-react'
import { useState, type ComponentProps } from 'react'
import { cn } from '@/lib/utils'
import { defaultUiLabels } from './default-labels'
import { Input } from './input'
import { useUiLabels } from './labels-context'

export function PasswordInput({ className, disabled, ...props }: Omit<ComponentProps<'input'>, 'type'>) {
  const [visible, setVisible] = useState(false)
  const labels = useUiLabels()
  const fallback = defaultUiLabels()
  const label = visible ? (labels.hidePassword ?? fallback.hidePassword) : (labels.showPassword ?? fallback.showPassword)
  const Icon = visible ? EyeOff : Eye
  return (
    <div className="relative">
      <Input {...props} disabled={disabled} type={visible ? 'text' : 'password'} className={cn('pr-11', className)} />
      <button
        type="button"
        onClick={() => setVisible((current) => !current)}
        disabled={disabled}
        aria-label={label}
        aria-pressed={visible}
        title={label}
        className="absolute inset-y-0 right-0 flex w-10 items-center justify-center rounded-r-(--input-radius) text-muted-foreground transition-colors outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50"
      >
        <Icon className="size-4" aria-hidden />
      </button>
    </div>
  )
}
