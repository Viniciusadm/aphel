import { useQuery, useQueryClient } from '@tanstack/react-query'
import { CalendarClock, FileText, Home, Hourglass, Menu, Plus } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { errorMessage } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { PageLoader } from '@/components/ui/page-loader'
import { StatusBadge } from '@/components/ui/status-badge'
import { DOCUMENT_STATE_TONES as STATE_TONES, formatDateTime, useBlocksValidation, useRecordFormatting } from '@/sdk'
import { pageKeys, pagePath, pageState, pageTarget, pagesApi, publicationApi, siteApi, siteKeys } from './api'
import { NewPageDialog } from './NewPageDialog'
import { PageWireframe, blockTypes } from './PageWireframe'
import './pages.css'

export function PagesListPage() {
  const { t } = useTranslation('site')
  const navigate = useNavigate()
  const client = useQueryClient()
  const structure = useQuery({ queryKey: siteKeys.structure, queryFn: siteApi.structure })
  const { hasPermission } = useSession()
  const formatting = useRecordFormatting()
  const [creating, setCreating] = useState(false)
  const pages = useQuery({ queryKey: pageKeys.list(), queryFn: pagesApi.list })
  const ids = (pages.data ?? []).map((page) => page.id)
  const statuses = useQuery({ queryKey: pageKeys.statuses(ids), queryFn: () => publicationApi.statuses(ids), enabled: ids.length > 0 })
  const openOf = (id: string) => statuses.data?.find((status) => status.record_id === id)?.open_request ?? null
  const canCreate = hasPermission('site.page.create')
  const validation = useBlocksValidation()

  const create = canCreate ? (
    <Button type="button" size="sm" disabled={!validation.isSuccess} onClick={() => setCreating(true)}>
      <Plus aria-hidden />
      {t('pages.list.new')}
    </Button>
  ) : null

  return (
    <Page>
      <PageHeader title={t('pages.list.title')} description={t('pages.list.description')} actions={create} />
      {pages.isPending ? (
        <PageLoader />
      ) : pages.isError ? (
        <ErrorState title={errorMessage(pages.error)} />
      ) : pages.data.length === 0 ? (
        <EmptyState icon={FileText} title={t('pages.list.emptyTitle')} description={t('pages.list.emptyDescription')} action={create ?? undefined} />
      ) : (
        <ul className="pages-grid">
          {pages.data.map((page) => {
            const state = pageState(page)
            const open = openOf(page.id)
            return (
              <li key={page.id} className="pages-card" onClick={() => navigate(`/site/pages/${page.id}`)}>
                <PageWireframe types={blockTypes(page.draft_blocks)} />
                <span className="pages-card-body">
                  <Link to={`/site/pages/${page.id}`} className="editorial-title-link pages-card-title" aria-label={t('pages.list.open', { title: page.draft_title })} onClick={(event) => event.stopPropagation()}>
                    {page.draft_title}
                  </Link>
                  <span className="editorial-address pages-card-address">{pagePath(page)}</span>
                  <span className="pages-card-tags">
                    <StatusBadge tone={STATE_TONES[state]}>{t(`pages.state.${state}`)}</StatusBadge>
                    {structure.data?.home === pageTarget(page.id) && (
                      <StatusBadge tone="primary" icon={<Home aria-hidden />}>
                        {t('pages.list.home')}
                      </StatusBadge>
                    )}
                    {open?.status === 'awaiting_approval' && (
                      <StatusBadge tone="warning" icon={<Hourglass aria-hidden />}>
                        {t('pages.review.badge')}
                      </StatusBadge>
                    )}
                    {open?.status === 'scheduled' && open.run_at && (
                      <StatusBadge tone="primary" icon={<CalendarClock aria-hidden />}>
                        {t('pages.schedule.badge', { date: formatDateTime(open.run_at, formatting) })}
                      </StatusBadge>
                    )}
                    {structure.data?.menu.includes(pageTarget(page.id)) && (
                      <StatusBadge tone="neutral" icon={<Menu aria-hidden />}>
                        {t('pages.list.inMenu')}
                      </StatusBadge>
                    )}
                  </span>
                  <span className="pages-card-updated">
                    {t('pages.list.columnUpdated')}: {page.updated_at ? formatDateTime(page.updated_at, formatting) : '—'}
                  </span>
                </span>
              </li>
            )
          })}
        </ul>
      )}
      {validation.isSuccess && (
        <NewPageDialog
          open={creating}
          onOpenChange={setCreating}
          onCreated={(page) => {
            void client.invalidateQueries({ queryKey: pageKeys.list() })
            navigate(`/site/pages/${page.id}`)
          }}
        />
      )}
    </Page>
  )
}
