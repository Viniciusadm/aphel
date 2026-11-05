import { CalendarDays, Menu, Search, type LucideIcon } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { NavLink } from 'react-router'
import { cn } from '@/lib/utils'
import { useAvailableSidebarEntries } from '@/app/layout/sidebar-entries'
import { TODAY_PATH } from '../routes'

interface NavTarget {
  key: 'today' | 'search'
  to: string
  icon: LucideIcon
}

const LINKS: NavTarget[] = [
  { key: 'today', to: TODAY_PATH, icon: CalendarDays },
  { key: 'search', to: '/search', icon: Search },
]

const ITEM_CLASS =
  'flex min-h-14 flex-1 flex-col items-center justify-center gap-0.5 text-[0.6875rem] font-medium text-muted-foreground transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset'

function NavItem({ target }: { target: NavTarget }) {
  const { t } = useTranslation('mobile')
  const Icon = target.icon
  return (
    <NavLink to={target.to} className={({ isActive }) => cn(ITEM_CLASS, isActive && 'text-primary')}>
      <Icon className="size-5" aria-hidden="true" />
      {t(`nav.${target.key}`)}
    </NavLink>
  )
}

export interface MobileNavProps {
  onOpenMenu: () => void
}

export function MobileNav({ onOpenMenu }: MobileNavProps) {
  const { t } = useTranslation('mobile')
  const [today, search] = LINKS
  const pinned = useAvailableSidebarEntries().find((entry) => entry.mobile)
  return (
    <nav
      aria-label={t('nav.label')}
      className="fixed inset-x-0 bottom-0 z-40 border-t bg-background/95 pb-[env(safe-area-inset-bottom)] backdrop-blur supports-[backdrop-filter]:bg-background/80 md:hidden print:hidden"
    >
      <div className="mx-auto flex max-w-lg items-stretch px-1">
        <NavItem target={today} />
        {pinned && (
          <NavLink to={pinned.to} className={({ isActive }) => cn(ITEM_CLASS, isActive && 'text-primary')}>
            {pinned.icon}
            {pinned.label}
          </NavLink>
        )}
        <NavItem target={search} />
        <button type="button" onClick={onOpenMenu} className={ITEM_CLASS} aria-haspopup="dialog">
          <Menu className="size-5" aria-hidden="true" />
          {t('nav.more')}
        </button>
      </div>
    </nav>
  )
}
