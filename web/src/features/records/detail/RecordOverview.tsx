import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useSearchParams } from 'react-router'
import { ArrowUpRight } from 'lucide-react'
import { groupTabs } from './mobile/tab-groups'
import type { RecordTab } from './types'

export function RecordOverview({ sections, fields }: { sections: RecordTab[]; fields?: ReactNode }) {
  const { t } = useTranslation('records-detail')
  const [params] = useSearchParams()
  const summaries = ['tasks', 'timeline'].flatMap((key) => {
    const section = sections.find((item) => item.key === key)
    return section?.summary ? [section] : []
  })
  return <div className="grid min-w-0 gap-6">
    {summaries.map((section, index) => {
      const next = new URLSearchParams(params)
      next.set('tab', section.key)
      return <div key={section.key} className="grid min-w-0 gap-6">{index === 1 && fields}<section className="grid min-w-0 gap-4 rounded-xl border bg-card p-4 sm:p-5">
        <div className="flex items-center justify-between gap-3">
          <h2 className="text-base font-semibold">{section.key === 'tasks' ? t('overview.nextActions') : section.key === 'timeline' ? t('overview.recentActivity') : section.label}</h2>
          <Link to={{ search: `?${next}` }} className="inline-flex min-h-11 items-center gap-1 text-sm font-medium text-primary">{t('overview.viewAll')}<ArrowUpRight className="size-4" aria-hidden /></Link>
        </div>
        {section.summary}
      </section></div>
    })}
    {summaries.length < 2 && fields}
    <nav aria-label={t('mobile.sections')} className="grid gap-5">
      {groupTabs(sections.filter((section) => section.key !== 'overview')).map(({ group, tabs }) => <section key={group} className="grid gap-2">
        <h2 className="text-sm font-semibold text-muted-foreground">{t(`mobile.groups.${group}`)}</h2>
        <div className="grid gap-2 sm:grid-cols-2">{tabs.map((section) => {
          const next = new URLSearchParams(params)
          next.set('tab', section.key)
          const Icon = section.icon
          return <Link key={section.key} to={{ search: `?${next}` }} className="flex min-h-14 items-center gap-3 rounded-lg border bg-card px-4 py-3 text-sm font-medium hover:bg-accent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">{Icon && <Icon className="size-4 text-muted-foreground" aria-hidden />}<span className="flex-1">{section.label}</span><ArrowUpRight className="size-4 text-muted-foreground" aria-hidden /></Link>
        })}</div>
      </section>)}
    </nav>
  </div>
}
