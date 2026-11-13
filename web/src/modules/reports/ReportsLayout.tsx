import { useTranslation } from 'react-i18next'
import { Outlet } from 'react-router'
import { ReportCard } from '@/components/ui/report-card'
import { Slot } from '@/kernel/modules/Slot'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function ReportsLayout() {
  return <Page><Outlet /></Page>
}

export function ReportsIndex() {
  const { t } = useTranslation('reports')
  return <>
    <PageHeader title={t('title')} description={t('description')} />
    <nav aria-label={t('library.title')} className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
      <ReportCard to="saved" title={t('tabs.saved')} description={t('library.saved')} />
      <Slot name="reports.library" props={{}} />
    </nav>
  </>
}
