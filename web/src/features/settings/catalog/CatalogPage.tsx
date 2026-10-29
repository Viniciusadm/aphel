import { BookOpen, Download, ExternalLink, PackageX, Puzzle, RefreshCw } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { ListSkeleton } from '@/features/settings/components/ListSkeleton'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { SectionHeader } from '@/features/settings/components/SectionHeader'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { useInvalidateVersions } from '../config-packages/versioning/api'
import { ComparisonView } from '../config-packages/versioning/ComparisonView'
import type { VersionComparison } from '../config-packages/versioning/types'
import { catalogApi, useCatalog } from './api'
import type { Catalog, CatalogEntry } from './types'

const KNOWN_ERRORS = [
  'catalog_disabled',
  'catalog_unavailable',
  'catalog_entry_not_found',
  'catalog_entry_not_installable',
  'aphel_too_old',
  'checksum_mismatch',
  'catalog_package_mismatch',
] as const
type KnownError = (typeof KNOWN_ERRORS)[number]

function knownError(error: unknown): KnownError | null {
  if (!isApiError(error)) return null
  return KNOWN_ERRORS.find((code) => code === error.code) ?? null
}

export function CatalogPage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <CatalogSection />
    </RequirePermission>
  )
}

interface Selected {
  entry: CatalogEntry
  comparison: VersionComparison
}

function CatalogSection() {
  const { t } = useTranslation('catalog')
  const catalog = useCatalog()
  const invalidate = useInvalidateVersions()
  const [selected, setSelected] = useState<Selected | null>(null)
  const [busy, setBusy] = useState<string | null>(null)
  const [failure, setFailure] = useState<{ id: string; message: string } | null>(null)
  const [notice, setNotice] = useState<string | null>(null)

  const describeError = (error: unknown) => {
    const code = knownError(error)
    if (code) return t(`errors.${code}`, { detail: isApiError(error) ? error.message : '' })
    return isApiError(error) ? error.message : t('errors.generic')
  }

  const compare = async (entry: CatalogEntry) => {
    setBusy(entry.id)
    setFailure(null)
    try {
      setSelected({ entry, comparison: await catalogApi.compare(entry.id) })
    } catch (error) {
      setFailure({ id: entry.id, message: describeError(error) })
    } finally {
      setBusy(null)
    }
  }

  if (selected) {
    return (
      <ComparisonView
        comparison={selected.comparison}
        notice={notice}
        refreshing={busy !== null}
        onBack={() => {
          setSelected(null)
          setNotice(null)
        }}
        onApply={(fingerprint, resolutions) => catalogApi.apply(selected.entry.id, fingerprint, resolutions)}
        onOutdated={async () => {
          setNotice(t('outdated'))
          await compare(selected.entry)
        }}
        onApplied={() => {
          setSelected(null)
          setNotice(null)
          void invalidate()
          void catalog.refetch()
        }}
      />
    )
  }

  return (
    <section className="flex min-w-0 flex-col gap-4">
      <SectionHeader
        title={t('page.title')}
        description={t('page.description')}
        action={
          <Button variant="outline" size="sm" disabled={catalog.isFetching} onClick={() => void catalog.refetch()}>
            <RefreshCw aria-hidden="true" />
            {t('page.refresh')}
          </Button>
        }
      />
      {catalog.isPending ? (
        <ListSkeleton rows={4} />
      ) : catalog.isError ? (
        <ErrorState
          title={t('page.unavailableTitle')}
          description={describeError(catalog.error)}
          onRetry={() => void catalog.refetch()}
        />
      ) : !catalog.data.enabled ? (
        <EmptyState icon={PackageX} title={t('page.disabledTitle')} description={t('page.disabledDescription')} />
      ) : (
        <CatalogContent
          catalog={catalog.data}
          busy={busy}
          failure={failure}
          onCompare={(entry) => void compare(entry)}
        />
      )}
    </section>
  )
}

interface CatalogContentProps {
  catalog: Catalog
  busy: string | null
  failure: { id: string; message: string } | null
  onCompare: (entry: CatalogEntry) => void
}

function CatalogContent({ catalog, busy, failure, onCompare }: CatalogContentProps) {
  const { t } = useTranslation('catalog')
  const templates = catalog.entries.filter((entry) => entry.kind === 'template')
  const modules = catalog.entries.filter((entry) => entry.kind === 'module')

  return (
    <div className="grid min-w-0 gap-6">
      <div className="grid gap-1 text-sm text-muted-foreground">
        <p className="font-medium text-foreground">{catalog.name}</p>
        {catalog.description && <p>{catalog.description}</p>}
        <p className="break-all text-xs">
          {t('page.source', { source: catalog.source ?? '', version: catalog.aphel_version })}
        </p>
      </div>
      {catalog.rejected.length > 0 && (
        <details className="rounded-md border border-warning/40 bg-warning/10 p-3 text-sm">
          <summary className="cursor-pointer">{t('rejected.title', { count: catalog.rejected.length })}</summary>
          <ul className="mt-2 grid gap-1">
            {catalog.rejected.map((rejected) => (
              <li key={rejected.position} className="break-words">
                {t('rejected.item', { id: rejected.id ?? `#${rejected.position + 1}`, reason: rejected.reason })}
              </li>
            ))}
          </ul>
        </details>
      )}
      <section className="grid gap-2" aria-labelledby="catalog-templates">
        <h3 id="catalog-templates" className="text-base font-semibold">
          {t('templates.title')}
        </h3>
        <p className="text-sm text-muted-foreground">{t('templates.description')}</p>
        {templates.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t('templates.empty')}</p>
        ) : (
          <ul className="grid gap-2 lg:grid-cols-2">
            {templates.map((entry) => (
              <li key={entry.id}>
                <TemplateCard
                  entry={entry}
                  busy={busy === entry.id}
                  disabled={busy !== null}
                  error={failure?.id === entry.id ? failure.message : null}
                  onCompare={() => onCompare(entry)}
                />
              </li>
            ))}
          </ul>
        )}
      </section>
      <section className="grid gap-2" aria-labelledby="catalog-modules">
        <h3 id="catalog-modules" className="text-base font-semibold">
          {t('modules.title')}
        </h3>
        <p className="text-sm text-muted-foreground">{t('modules.description')}</p>
        {modules.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t('modules.empty')}</p>
        ) : (
          <ul className="grid gap-2 lg:grid-cols-2">
            {modules.map((entry) => (
              <li key={entry.id}>
                <ModuleCard entry={entry} />
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  )
}

function EntryHeader({ entry }: { entry: CatalogEntry }) {
  const { t } = useTranslation('catalog')
  return (
    <div className="grid min-w-0 gap-1">
      <div className="flex min-w-0 flex-wrap items-center gap-2">
        <h4 className="min-w-0 font-medium break-words">{entry.name}</h4>
        <Badge variant="secondary">{entry.version}</Badge>
        {!entry.compatible && <Badge variant="warning">{t('entry.incompatible', { version: entry.min_aphel_version })}</Badge>}
      </div>
      {entry.description && <p className="text-sm break-words text-muted-foreground">{entry.description}</p>}
      <p className="flex flex-wrap gap-x-3 gap-y-0.5 text-xs text-muted-foreground">
        <span>{t('entry.author', { author: entry.author })}</span>
        <span>{t('entry.license', { license: entry.license })}</span>
        <span>{t('entry.minVersion', { version: entry.min_aphel_version })}</span>
        <code className="break-all">{entry.id}</code>
      </p>
      {entry.tags.length > 0 && (
        <div className="flex flex-wrap gap-1">
          {entry.tags.map((tag) => (
            <Badge key={tag} variant="outline">
              {tag}
            </Badge>
          ))}
        </div>
      )}
    </div>
  )
}

function DocsLink({ url }: { url?: string | null }) {
  const { t } = useTranslation('catalog')
  if (!url) return null
  return (
    <Button variant="ghost" size="sm" asChild>
      <a href={url} target="_blank" rel="noreferrer noopener">
        <BookOpen aria-hidden="true" />
        {t('entry.documentation')}
        <ExternalLink className="size-3" aria-hidden="true" />
      </a>
    </Button>
  )
}

interface TemplateCardProps {
  entry: CatalogEntry
  busy: boolean
  disabled: boolean
  error: string | null
  onCompare: () => void
}

function TemplateCard({ entry, busy, disabled, error, onCompare }: TemplateCardProps) {
  const { t } = useTranslation('catalog')
  const action = !entry.installed_version ? 'install' : entry.update_available ? 'update' : 'review'
  return (
    <article className="grid h-full min-w-0 content-between gap-3 rounded-lg border bg-card p-3 sm:p-4" aria-label={entry.name}>
      <EntryHeader entry={entry} />
      <div className="grid gap-2">
        {entry.installed_version && (
          <p className="text-xs">
            {entry.update_available
              ? t('templates.updateAvailable', { installed: entry.installed_version, version: entry.version })
              : t('templates.installed', { version: entry.installed_version })}
          </p>
        )}
        {error && (
          <p role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 p-2 text-sm break-words">
            {error}
          </p>
        )}
        <div className="flex flex-wrap gap-2">
          <Button size="sm" disabled={disabled || !entry.compatible} onClick={onCompare}>
            <Download aria-hidden="true" />
            {busy ? t('templates.checking') : t(`templates.actions.${action}`)}
          </Button>
          <DocsLink url={entry.documentation_url} />
        </div>
      </div>
    </article>
  )
}

function ModuleCard({ entry }: { entry: CatalogEntry }) {
  const { t } = useTranslation('catalog')
  const module = entry.module
  return (
    <article className="grid h-full min-w-0 content-between gap-3 rounded-lg border bg-card p-3 sm:p-4" aria-label={entry.name}>
      <div className="flex min-w-0 gap-3">
        <Puzzle className="mt-0.5 size-5 shrink-0 text-muted-foreground" aria-hidden="true" />
        <EntryHeader entry={entry} />
      </div>
      {module && (
        <div className="grid gap-2 text-sm">
          <p className="text-muted-foreground">{t('modules.howTo')}</p>
          <pre className="min-w-0 overflow-x-auto rounded-md bg-muted p-2 font-mono text-xs">
            {t('modules.command', { key: module.key, version: entry.version })}
          </pre>
          {module.install_notes && <p className="break-words text-muted-foreground">{module.install_notes}</p>}
          <div className="flex flex-wrap gap-2">
            <DocsLink url={entry.documentation_url} />
            {module.repository && (
              <Button variant="ghost" size="sm" asChild>
                <a href={module.repository} target="_blank" rel="noreferrer noopener">
                  {t('modules.repository')}
                  <ExternalLink className="size-3" aria-hidden="true" />
                </a>
              </Button>
            )}
          </div>
        </div>
      )}
    </article>
  )
}
