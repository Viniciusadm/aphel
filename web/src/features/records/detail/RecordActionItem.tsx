import type { LucideIcon } from 'lucide-react'
import { useContext } from 'react'
import { cn } from '@/lib/utils'
import { RecordActionsContext } from './actions-context'

export interface RecordActionItemProps {
  icon?: LucideIcon
  label: string
  onSelect: () => void
  destructive?: boolean
  disabled?: boolean
}

export function RecordActionItem({ icon: Icon, label, onSelect, destructive = false, disabled = false }: RecordActionItemProps) {
  const { close } = useContext(RecordActionsContext)
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={() => {
        close()
        onSelect()
      }}
      className={cn(
        'flex min-h-11 w-full items-center gap-2 rounded-md px-3 text-left text-sm outline-none hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-50',
        destructive && 'text-destructive hover:bg-destructive/10',
      )}
    >
      {Icon && <Icon className="size-4 shrink-0" aria-hidden />}
      {label}
    </button>
  )
}

export function RecordActionsDivider() {
  return <div role="separator" className="-mx-1 my-1 h-px bg-border" />
}
