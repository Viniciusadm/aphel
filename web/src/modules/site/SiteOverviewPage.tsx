import { CheckCircle2, Circle, ExternalLink } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { ItemList, ItemRow } from '@/components/ui/item-list'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Skeleton } from '@/components/ui/skeleton'
import { formatDateTime, useRecordFormatting } from '@/sdk'
import { type SiteStatus, useSiteStatus } from './api'

interface Step {
  key: string
  done: boolean
  title: string
  detail: string
  action?: { to: string; label: string }
}

function useSteps(status: SiteStatus): Step[] {
  const { t } = useTranslation('site')
  const published = status.content.reduce((total, item) => total + item.published, 0)
  const identity = status.named && status.has_logo
  const home = status.home !== null
  const menu = status.menu_items > 0
  return [
    {
      key: 'identity',
      done: identity,
      title: t('overview.steps.identity.title'),
      detail: identity ? t('overview.steps.identity.done') : t('overview.steps.identity.todo'),
      action: { to: '/site/appearance', label: t('overview.steps.identity.action') },
    },
    {
      key: 'home',
      done: home,
      title: t('overview.steps.home.title'),
      detail: home ? t('overview.steps.home.done') : t('overview.steps.home.todo'),
      action: { to: '/site/navigation', label: t('overview.steps.home.action') },
    },
    {
      key: 'menu',
      done: menu,
      title: t('overview.steps.menu.title'),
      detail: menu ? t('overview.steps.menu.done') : t('overview.steps.menu.todo'),
      action: { to: '/site/navigation', label: t('overview.steps.menu.action') },
    },
    {
      key: 'content',
      done: published > 0,
      title: t('overview.steps.content.title'),
      detail: published > 0 ? t('overview.steps.content.done') : t('overview.steps.content.todo'),
    },
    {
      key: 'indexing',
      done: status.indexing,
      title: t('overview.steps.indexing.title'),
      detail: status.indexing ? t('overview.steps.indexing.done') : t('overview.steps.indexing.todo'),
      action: { to: '/site/general', label: t('overview.steps.indexing.action') },
    },
  ]
}

function StepsCard({ status }: { status: SiteStatus }) {
  const { t } = useTranslation('site')
  const steps = useSteps(status)
  const done = steps.filter((step) => step.done).length
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('overview.steps.title')}</CardTitle>
        <CardDescription>{t('overview.steps.progress', { done, total: steps.length })}</CardDescription>
      </CardHeader>
      <CardContent>
        <ItemList>
          {steps.map((step) => (
            <ItemRow key={step.key} className="justify-between">
              <span className="flex min-w-0 items-start gap-3">
                {step.done ? (
                  <CheckCircle2 className="mt-0.5 size-4 shrink-0 text-success" aria-label={t('overview.steps.done')} />
                ) : (
                  <Circle className="mt-0.5 size-4 shrink-0 text-muted-foreground" aria-label={t('overview.steps.pending')} />
                )}
                <span className="grid gap-0.5">
                  <span className="font-medium">{step.title}</span>
                  <span className="text-muted-foreground">{step.detail}</span>
                </span>
              </span>
              {step.action && !step.done && (
                <Button asChild variant="outline" size="sm">
                  <Link to={step.action.to}>{step.action.label}</Link>
                </Button>
              )}
            </ItemRow>
          ))}
        </ItemList>
      </CardContent>
    </Card>
  )
}

function ContentCard({ status }: { status: SiteStatus }) {
  const { t } = useTranslation('site')
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('overview.content.title')}</CardTitle>
        <CardDescription>{t('overview.content.description')}</CardDescription>
      </CardHeader>
      <CardContent>
        {status.content.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t('overview.content.empty')}</p>
        ) : (
          <ItemList>
            {status.content.map((item) => (
              <ItemRow key={item.record_type} className="justify-between">
                <span className="font-medium">{item.label}</span>
                <span className="text-muted-foreground">
                  {t('overview.content.published', { count: item.published })} · {t('overview.content.drafts', { count: item.drafts })}
                </span>
              </ItemRow>
            ))}
          </ItemList>
        )}
      </CardContent>
    </Card>
  )
}

function RecentCard({ status }: { status: SiteStatus }) {
  const { t } = useTranslation('site')
  const formatting = useRecordFormatting()
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('overview.recent.title')}</CardTitle>
      </CardHeader>
      <CardContent>
        {status.recent.length === 0 ? (
          <EmptyState title={t('overview.recent.empty')} description={t('overview.recent.emptyDescription')} />
        ) : (
          <ItemList>
            {status.recent.map((item) => (
              <ItemRow key={`${item.record_type}:${item.record_id}`} className="justify-between">
                <a href={item.url} target="_blank" rel="noreferrer" className="min-w-0 truncate font-medium hover:underline">
                  {item.title}
                </a>
                {item.published_at && <span className="text-muted-foreground">{formatDateTime(item.published_at, formatting)}</span>}
              </ItemRow>
            ))}
          </ItemList>
        )}
      </CardContent>
    </Card>
  )
}

export function SiteOverviewPage() {
  const { t } = useTranslation('site')
  const status = useSiteStatus()
  return (
    <Page>
      <PageHeader
        title={t('overview.title')}
        description={status.data ? t('overview.address', { url: status.data.base_url }) : t('overview.description')}
        actions={
          status.data && (
            <Button asChild variant="outline">
              <a href={`${status.data.base_url.replace(/\/$/, '')}/`} target="_blank" rel="noreferrer">
                {t('page.open')}
                <ExternalLink aria-hidden="true" />
              </a>
            </Button>
          )
        }
      />
      {status.isPending ? (
        <div className="grid gap-4">
          <Skeleton className="h-48" />
          <Skeleton className="h-32" />
        </div>
      ) : status.isError || !status.data ? (
        <ErrorState onRetry={() => void status.refetch()} />
      ) : (
        <div className="grid gap-6">
          {status.data.home && (
            <p className="text-sm text-muted-foreground">
              {t('overview.home', { label: status.data.home.label || status.data.home.url })}
            </p>
          )}
          <StepsCard status={status.data} />
          <div className="grid gap-6 lg:grid-cols-2">
            <ContentCard status={status.data} />
            <RecentCard status={status.data} />
          </div>
        </div>
      )}
    </Page>
  )
}
