import type { ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { ErrorState } from '@/components/ui/error-state'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Skeleton } from '@/components/ui/skeleton'
import { type OrganizationConfiguration, useOrganizationConfiguration } from '@/sdk'
import { type SiteOverview, useSiteOverview } from './api'
import {
  AddressCard,
  AppearanceCard,
  CacheCard,
  CustomersCard,
  LanguagesCard,
  MenuCard,
  PathRedirectsCard,
  RedirectsCard,
  SiteSummary,
} from './SiteCards'
import { AppearanceEditor } from './AppearanceEditor'

type ScreenKey = 'general' | 'frame' | 'appearance' | 'languages' | 'customers' | 'redirects'

function SiteScreen({
  screen,
  children,
}: {
  screen: ScreenKey
  children: (configuration: OrganizationConfiguration, overview: SiteOverview) => ReactNode
}) {
  const { t } = useTranslation('site')
  const configuration = useOrganizationConfiguration()
  const overview = useSiteOverview()
  const loading = configuration.isPending || overview.isPending
  const failed = configuration.isError || overview.isError

  return (
    <Page>
      <PageHeader title={t(`screens.${screen}.title`)} description={t(`screens.${screen}.description`)} />
      {loading ? (
        <div className="grid gap-4">
          <Skeleton className="h-40" />
          <Skeleton className="h-40" />
        </div>
      ) : failed || !configuration.data || !overview.data ? (
        <ErrorState
          onRetry={() => {
            void configuration.refetch()
            void overview.refetch()
          }}
        />
      ) : (
        <div className="grid gap-6">{children(configuration.data, overview.data)}</div>
      )}
    </Page>
  )
}

export function SiteGeneralPage() {
  return (
    <SiteScreen screen="general">
      {(configuration, overview) => (
        <>
          <SiteSummary overview={overview} />
          <AddressCard configuration={configuration} />
          <AppearanceCard configuration={configuration} overview={overview} />
          <CacheCard />
        </>
      )}
    </SiteScreen>
  )
}

export function SiteFramePage() {
  return (
    <SiteScreen screen="frame">
      {(configuration, overview) => (
        <>
          <AppearanceEditor mode="frame" />
          <MenuCard key={overview.menu_items.map((item) => item.id).join()} configuration={configuration} overview={overview} />
        </>
      )}
    </SiteScreen>
  )
}

export function SiteAppearancePage() {
  const { t } = useTranslation('site')
  return <Page><PageHeader title={t('screens.appearance.title')} description={t('screens.appearance.description')} /><AppearanceEditor mode="style" /></Page>
}

export function SiteLanguagesPage() {
  return <SiteScreen screen="languages">{(configuration, overview) => <LanguagesCard configuration={configuration} overview={overview} />}</SiteScreen>
}

export function SiteCustomersPage() {
  return <SiteScreen screen="customers">{(configuration, overview) => <CustomersCard configuration={configuration} overview={overview} />}</SiteScreen>
}

export function SiteRedirectsPage() {
  return (
    <SiteScreen screen="redirects">
      {() => (
        <>
          <PathRedirectsCard />
          <RedirectsCard />
        </>
      )}
    </SiteScreen>
  )
}
