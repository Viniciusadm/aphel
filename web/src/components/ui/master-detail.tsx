import type { ReactNode } from 'react'
import { Button } from './button'

export interface MasterItem {
  id: string
  label: ReactNode
  actions?: ReactNode
}

export interface MasterDetailProps {
  items: MasterItem[]
  selected: string | null
  onSelect: (id: string) => void
  header?: ReactNode
  detail: ReactNode
}

export function MasterDetail({ items, selected, onSelect, header, detail }: MasterDetailProps) {
  return (
    <div data-slot="master-detail" className="grid min-w-0 gap-4 md:grid-cols-[16rem_minmax(0,1fr)]">
      <nav className="grid min-w-0 content-start gap-1">
        {header}
        {items.map((item) => (
          <div key={item.id} className="flex min-w-0 items-center gap-1">
            <Button
              className="min-w-0 flex-1 justify-start"
              size="sm"
              variant={item.id === selected ? 'secondary' : 'ghost'}
              aria-current={item.id === selected ? 'true' : undefined}
              onClick={() => onSelect(item.id)}
            >
              <span className="min-w-0 truncate">{item.label}</span>
            </Button>
            {item.actions}
          </div>
        ))}
      </nav>
      <div className="min-w-0">{detail}</div>
    </div>
  )
}
