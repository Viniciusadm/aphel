import { useTranslation } from 'react-i18next'
import { Outlet } from 'react-router'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function CallsLayout() {
  const { t } = useTranslation('telephony')
  return (
    <Page>
      <PageHeader title={t('page.title')} description={t('page.description')} />
      <Outlet />
    </Page>
  )
}
