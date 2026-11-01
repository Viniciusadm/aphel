import type { ReactNode } from 'react'
import { useSearchParams } from 'react-router'
import { SectionWorkspace, SectionContent, SectionIndex, SectionLink } from './section-workspace'

export interface UrlTab {
  key: string
  label: string
  content: ReactNode
}

export function UrlTabs({ tabs, param = 'tab' }: { tabs: UrlTab[]; param?: string }) {
  const [params, setParams] = useSearchParams()
  const requested = params.get(param)
  const active = tabs.find((tab) => tab.key === requested)?.key ?? tabs[0]?.key
  const select = (key: string) => {
    const next = new URLSearchParams(params)
    if (key === tabs[0]?.key) next.delete(param)
    else next.set(param, key)
    setParams(next)
  }
  if (tabs.length === 0) return null
  return (
    <SectionWorkspace value={active} onValueChange={select} >
      <SectionIndex>
        {tabs.map((tab) => (
          <SectionLink key={tab.key} value={tab.key}>
            {tab.label}
          </SectionLink>
        ))}
      </SectionIndex>
      {tabs.map((tab) => (
        <SectionContent key={tab.key} value={tab.key} >
          {tab.content}
        </SectionContent>
      ))}
    </SectionWorkspace>
  )
}
