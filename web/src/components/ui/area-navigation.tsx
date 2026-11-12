import { useNavigate } from 'react-router'
import { NativeSelect } from './native-select'
import { cn } from '@/lib/utils'
import { tabsListVariants, tabsTriggerVariants } from './tabs-variants'
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
    <div className={cn(tabsListVariants({ variant: 'line' }), 'hidden flex-wrap sm:flex')}>
      {items.map((item) => <NavLink key={item.to} to={item.to} end={item.end} className={tabsTriggerVariants({ variant: 'line' })}>{item.label}</NavLink>)}
    </div>
  </nav>
}
