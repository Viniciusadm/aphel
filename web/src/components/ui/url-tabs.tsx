import type { ReactNode } from 'react'
import { useSearchParams } from 'react-router'
import { Tabs, TabsContent, TabsList, TabsTrigger } from './tabs'

export interface UrlTab {
  key: string
  label: string
  content: ReactNode
}

export function UrlTabs({ tabs, param = 'tab' }: { tabs: UrlTab[]; param?: string }) {
  const [params, setParams] = useSearchParams()
  const requested = params.get(param)
  const active = tabs.find((tab) => tab.key === requested)?.key ?? tabs[0].key
  const select = (key: string) => {
    const next = new URLSearchParams(params)
    if (key === tabs[0].key) next.delete(param)
    else next.set(param, key)
    setParams(next, { replace: true })
  }
  return (
    <Tabs value={active} onValueChange={select} className="flex min-w-0 flex-col gap-6">
      <TabsList>
        {tabs.map((tab) => (
          <TabsTrigger key={tab.key} value={tab.key}>
            {tab.label}
          </TabsTrigger>
        ))}
      </TabsList>
      {tabs.map((tab) => (
        <TabsContent key={tab.key} value={tab.key} className="flex min-w-0 flex-col gap-6">
          {tab.content}
        </TabsContent>
      ))}
    </Tabs>
  )
}
