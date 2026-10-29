import { useTranslation } from 'react-i18next'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
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
  if (tabs.length === 0) return null

  return (
    <Tabs value={active} onValueChange={setActive} className={cn('grid min-w-0 gap-4', className)}>
      <TabsList aria-label={t('tabs.label')} className="scroll-smooth-thin h-11 w-full justify-start sm:h-9 sm:w-fit">
        {tabs.map((tab) => {
          const Icon = tab.icon
          return (
            <TabsTrigger key={tab.key} value={tab.key} className="px-2.5 sm:px-3">
              {Icon && <Icon className="size-4" aria-hidden="true" />}
              {tab.label}
            </TabsTrigger>
          )
        })}
      </TabsList>
      {tabs.map((tab) => (
        <TabsContent key={tab.key} value={tab.key} className="min-w-0">
          {tab.content}
        </TabsContent>
      ))}
    </Tabs>
  )
}
