import { useTranslation } from 'react-i18next'
import { Link, useSearchParams } from 'react-router'
import { NativeSelect } from '@/components/ui/native-select'
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
  const overviewParams = new URLSearchParams(params)
  overviewParams.delete(param ?? 'tab')
  if (tabs.length === 0) return null

  return (
    <div className={cn('grid min-w-0 gap-5', className)}>
      <div className="flex flex-wrap items-center justify-between gap-3 border-b pb-4">
        <Link to={{ search: `?${overviewParams}` }} className="inline-flex min-h-11 items-center text-sm font-medium text-muted-foreground hover:text-foreground">{t('overview.title')}</Link>
        <NativeSelect className="w-full sm:w-56" aria-label={t('tabs.label')} value={active} onChange={(event) => setActive(event.target.value)}>
          {tabs.map((tab) => <option key={tab.key} value={tab.key}>{tab.label}</option>)}
        </NativeSelect>
      </div>
      <section key={active} className="min-w-0" aria-label={tabs.find((tab) => tab.key === active)?.label}>
        {tabs.find((tab) => tab.key === active)?.content}
      </section>
    </div>
  )
}
