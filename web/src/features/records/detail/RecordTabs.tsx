import { useTranslation } from 'react-i18next'
import { Link, useSearchParams } from 'react-router'
import { ChevronDown } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { cn } from '@/lib/utils'
import type { RecordTab } from './types'
import { useActiveTab } from './use-active-tab'

export interface RecordTabsProps {
  tabs: RecordTab[]
  defaultTab?: string
  param?: string
  className?: string
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

  return (
    <div className={cn('grid min-w-0 gap-5', className)}>
      <nav aria-label={t('tabs.label')} className="flex flex-wrap items-center gap-2 border-b pb-2">
        {tabs.filter((tab) => ['overview', 'tasks', 'timeline'].includes(tab.key)).map((tab) => {
          const next = new URLSearchParams(params)
          next.set(param ?? 'tab', tab.key)
          return <Link key={tab.key} to={{ search: `?${next}` }} aria-current={active === tab.key ? 'page' : undefined} className={cn('inline-flex min-h-11 items-center rounded-md px-3 text-sm hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring', active === tab.key ? 'bg-accent font-semibold' : 'text-muted-foreground')}>{tab.label}</Link>
        })}
        {tabs.some((tab) => !['overview', 'tasks', 'timeline'].includes(tab.key)) && <DropdownMenu>
          <DropdownMenuTrigger asChild><Button variant="ghost" size="sm">
            {tabs.find((tab) => tab.key === active && !['overview', 'tasks', 'timeline'].includes(tab.key))?.label ?? t('mobile.sections')}<ChevronDown aria-hidden />
          </Button></DropdownMenuTrigger>
          <DropdownMenuContent align="start">{tabs.filter((tab) => !['overview', 'tasks', 'timeline'].includes(tab.key)).map((tab) => <DropdownMenuItem key={tab.key} onSelect={() => setActive(tab.key)}>{tab.label}</DropdownMenuItem>)}</DropdownMenuContent>
        </DropdownMenu>}
      </nav>
      <section key={active} className="min-w-0" aria-label={tabs.find((tab) => tab.key === active)?.label}>
        {tabs.find((tab) => tab.key === active)?.content}
      </section>
    </div>
  )
}
