import { useTranslation } from 'react-i18next'
import { Link, useSearchParams } from 'react-router'
import { ChevronDown } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { cn } from '@/lib/utils'
import { tabsListVariants, tabsTriggerVariants } from '@/components/ui/tabs-variants'
import type { RecordTab } from './types'
import { useActiveTab } from './use-active-tab'

export interface RecordTabsProps {
  tabs: RecordTab[]
  defaultTab?: string
  param?: string
  className?: string
}

const PRIMARY_TAB_KEYS = ['overview', 'tasks', 'timeline']

const MAX_VISIBLE_TABS = 5

function isPrimaryTab(tab: { key: string; group?: string }): boolean {
  return PRIMARY_TAB_KEYS.includes(tab.key) || tab.group === 'primary'
}

export function RecordTabs({ tabs, defaultTab, param, className }: RecordTabsProps) {
  const { t } = useTranslation('records-detail')
  const [active, setActive] = useActiveTab(
    tabs.map((tab) => tab.key),
    defaultTab,
    param,
  )
  const [params] = useSearchParams()
  if (tabs.length === 0) return null
  const showAll = tabs.length <= MAX_VISIBLE_TABS
  const visible = showAll ? tabs : tabs.filter(isPrimaryTab)
  const overflow = showAll ? [] : tabs.filter((tab) => !isPrimaryTab(tab))

  return (
    <div className={cn('grid min-w-0 gap-5', className)}>
      <nav aria-label={t('tabs.label')} className={cn(tabsListVariants({ variant: 'line' }), 'flex-wrap items-center')}>
        {visible.map((tab) => {
          const next = new URLSearchParams(params)
          next.set(param ?? 'tab', tab.key)
          return <Link key={tab.key} to={{ search: `?${next}` }} aria-current={active === tab.key ? 'page' : undefined} className={tabsTriggerVariants({ variant: 'line' })}>{tab.label}</Link>
        })}
        {overflow.length > 0 && <DropdownMenu>
          <DropdownMenuTrigger asChild><Button variant="ghost" size="sm" className="mb-2 self-center">
            {overflow.find((tab) => tab.key === active)?.label ?? t('tabs.more')}<ChevronDown aria-hidden />
          </Button></DropdownMenuTrigger>
          <DropdownMenuContent align="start">{overflow.map((tab) => <DropdownMenuItem key={tab.key} onSelect={() => setActive(tab.key)}>{tab.label}</DropdownMenuItem>)}</DropdownMenuContent>
        </DropdownMenu>}
      </nav>
      <section key={active} className="min-w-0" aria-label={tabs.find((tab) => tab.key === active)?.label}>
        {tabs.find((tab) => tab.key === active)?.content}
      </section>
    </div>
  )
}
