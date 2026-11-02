import { useTranslation } from 'react-i18next'
import { Navigate, Outlet, useLocation, useSearchParams } from 'react-router'
import { AreaNavigation } from '@/components/ui/area-navigation'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function SchedulingPage() {
  const { t } = useTranslation('scheduling')
  const location = useLocation()
  const [params] = useSearchParams()
  const legacy = params.get('tab')
  if (legacy && ['bookings', 'availability', 'meeting-types'].includes(legacy)) {
    const next = new URLSearchParams(params)
    next.delete('tab')
    const path = legacy === 'bookings' ? '/scheduling' : `/scheduling/${legacy}`
    return <Navigate replace to={{ pathname: path, search: next.toString() }} />
  }
  return <Page>
    <PageHeader title={t('page.title')} />
    <AreaNavigation label={t('page.title')} active={location.pathname} items={[
      { to: '/scheduling', label: t('tabs.bookings'), end: true },
      { to: '/scheduling/availability', label: t('tabs.availability') },
      { to: '/scheduling/meeting-types', label: t('tabs.meetingTypes') },
    ]} />
    <Outlet />
  </Page>
}
