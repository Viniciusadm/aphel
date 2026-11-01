import { useNavigate } from 'react-router'
import { NativeSelect } from './native-select'
import { cn } from '@/lib/utils'
import { NavLink } from 'react-router'

export interface AreaDestination {
  to: string
  label: string
  end?: boolean
}

export function AreaNavigation({ items, label, active }: { items: AreaDestination[]; label: string; active: string }) {
  const navigate = useNavigate()
  if (items.length < 2) return null
  return <nav aria-label={label}>
    <NativeSelect className="w-full sm:hidden" aria-label={label} value={active} onChange={(event) => void navigate(event.target.value)}>
      {items.map((item) => <option key={item.to} value={item.to}>{item.label}</option>)}
    </NativeSelect>
    <div className="hidden flex-wrap gap-2 sm:flex">
      {items.map((item) => <NavLink key={item.to} to={item.to} end={item.end} className={({ isActive }) => cn('inline-flex min-h-11 items-center rounded-md px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring', isActive ? 'bg-accent font-medium text-accent-foreground' : 'text-muted-foreground hover:bg-muted')}>{item.label}</NavLink>)}
    </div>
  </nav>
}
