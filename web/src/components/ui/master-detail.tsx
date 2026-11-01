import { useState, type ReactNode } from 'react'
import { ArrowLeft } from 'lucide-react'
import { Button } from './button'
import { cn } from '@/lib/utils'
import { useUiLabels } from './labels-context'

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
  const labels = useUiLabels()
  const [showDetail, setShowDetail] = useState(false)
  const hasSelection = selected !== null && items.some((item) => item.id === selected)
  return (
    <div data-slot="master-detail" className="grid min-w-0 gap-4 md:grid-cols-[16rem_minmax(0,1fr)]">
      <nav className={cn('min-w-0 content-start gap-1 md:grid', showDetail && hasSelection ? 'hidden' : 'grid')}>
        {header}
        {items.map((item) => (
          <div key={item.id} className="flex min-w-0 items-center gap-1">
            <Button
              className="min-w-0 flex-1 justify-start"
              size="sm"
              variant={item.id === selected ? 'secondary' : 'ghost'}
              aria-current={item.id === selected ? 'true' : undefined}
              onClick={() => { onSelect(item.id); setShowDetail(true) }}
            >
              <span className="min-w-0 truncate">{item.label}</span>
            </Button>
            {item.actions}
          </div>
        ))}
      </nav>
      <div className={cn('min-w-0 md:block', showDetail && hasSelection || selected === null ? 'block' : 'hidden')}>
        <Button variant="ghost" className="mb-3 md:hidden" onClick={() => setShowDetail(false)}><ArrowLeft aria-hidden />{labels.back}</Button>
        {detail}
      </div>
    </div>
  )
}
