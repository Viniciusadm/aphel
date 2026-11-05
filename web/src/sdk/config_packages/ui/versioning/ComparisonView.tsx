import { ArrowLeft, CircleAlert } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { PLAN_OUTDATED_CODE } from '../api'
import { usePackageLabels } from '../lib/labels'
import { useDateFormatter } from './format'
import type { ComparisonSummary, Resolution, Resolutions, VersionApplyResult, VersionComparison, VersionItem } from './types'
import { VersionItemCard } from './VersionItemCard'

const SUMMARY_ORDER = ['added', 'changed', 'removed', 'unchanged', 'locally_modified', 'conflicts'] as const

interface ComparisonViewProps {
  comparison: VersionComparison
  notice: string | null
  refreshing: boolean
  onBack: () => void
  onApply: (fingerprint: string, resolutions: Resolutions) => Promise<VersionApplyResult>
  onOutdated: () => Promise<void>
  onApplied: (result: VersionApplyResult) => void
}

interface SectionGroup {
  section: string
  relevant: VersionItem[]
  quiet: VersionItem[]
}

function isRelevant(item: VersionItem): boolean {
  return (
    item.package_status !== 'unchanged' ||
    item.locally_modified ||
    item.needs_resolution ||
    (item.plan != null && item.plan.action !== 'unchanged')
  )
}

function groupBySection(comparison: VersionComparison): SectionGroup[] {
  const order = [...comparison.sections, ...comparison.items.map((item) => item.section)]
  return [...new Set(order)]
    .map((section) => {
      const items = comparison.items.filter((item) => item.section === section)
      return {
        section,
        relevant: items.filter(isRelevant),
        quiet: items.filter((item) => !isRelevant(item)),
      }
    })
    .filter((group) => group.relevant.length + group.quiet.length > 0)
}

export function ComparisonView({
  comparison,
  notice,
  refreshing,
  onBack,
  onApply,
  onOutdated,
  onApplied,
}: ComparisonViewProps) {
  const { t } = useTranslation('config-package-versioning')
  const labels = usePackageLabels()
  const formatter = useDateFormatter()
  const [choices, setChoices] = useState<Resolutions>({})
  const [confirming, setConfirming] = useState(false)
  const [applying, setApplying] = useState(false)
  const groups = useMemo(() => groupBySection(comparison), [comparison])
  const pending = comparison.items.filter((item) => item.needs_resolution)

  const resolutions = useMemo(
    () =>
      Object.fromEntries(pending.map((item) => [item.id, choices[item.id] ?? 'keep_local'])) as Resolutions,
    [pending, choices],
  )
  const choose = (id: string, resolution: Resolution) => setChoices((current) => ({ ...current, [id]: resolution }))
  const identity = comparison.package
  const canApply = comparison.valid && identity != null && !refreshing

  const confirm = async () => {
    setApplying(true)
    try {
      const result = await onApply(comparison.fingerprint, resolutions)
      notifySuccess(t('apply.success', { name: result.version.name, version: result.version.version }), {
        description: t('apply.summary', {
          create: result.summary.create,
          update: result.summary.update,
          kept: result.kept_local.length,
        }),
      })
      onApplied(result)
    } catch (error) {
      if (isApiError(error) && error.code === PLAN_OUTDATED_CODE) {
        setConfirming(false)
        await onOutdated()
        return
      }
      throw error
    } finally {
      setApplying(false)
    }
  }

  return (
    <div className="grid min-w-0 gap-4">
      <div>
        <Button variant="ghost" size="sm" onClick={onBack}>
          <ArrowLeft aria-hidden="true" />
          {t('comparison.back')}
        </Button>
      </div>
      <Card>
        <CardHeader>
          <CardTitle className="flex flex-wrap items-center gap-2 break-words">
            {identity ? t('comparison.title', { name: identity.name, version: identity.version }) : t('comparison.untitled')}
            {comparison.relation && (
              <Badge variant={comparison.relation === 'downgrade' ? 'warning' : 'secondary'}>
                {t(`relation.${comparison.relation}`)}
              </Badge>
            )}
          </CardTitle>
          {identity?.description && <CardDescription className="break-words">{identity.description}</CardDescription>}
        </CardHeader>
        <CardContent className="grid gap-3">
          <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
            {identity && <span>{t('comparison.packageId', { id: identity.id })}</span>}
            {identity?.author && <span>{t('comparison.author', { author: identity.author })}</span>}
            {comparison.installed ? (
              <span>
                {t('comparison.installedVersion', {
                  version: comparison.installed.version,
                  date: formatter.format(new Date(comparison.installed.installed_at)),
                })}
              </span>
            ) : (
              comparison.valid && <span>{t('comparison.notInstalled')}</span>
            )}
          </div>
          {comparison.relation === 'downgrade' && (
            <p className="rounded-md border border-warning/40 bg-warning/10 p-2 text-sm">{t('comparison.downgradeWarning')}</p>
          )}
          {comparison.valid && <SummaryList summary={comparison.summary} />}
        </CardContent>
      </Card>

      <div role="status" aria-live="polite">
        {notice && (
          <p className="rounded-md border border-warning/40 bg-warning/10 p-3 text-sm">
            {refreshing ? t('comparison.refreshing') : notice}
          </p>
        )}
      </div>

      {!comparison.valid && (
        <div role="alert" className="grid gap-2 rounded-lg border border-destructive/30 bg-destructive/5 p-4">
          <p className="flex items-center gap-2 font-medium text-destructive">
            <CircleAlert className="size-4" aria-hidden="true" />
            {t('comparison.invalidTitle')}
          </p>
          <ul className="grid gap-1.5 text-sm" aria-label={t('comparison.issuesLabel')}>
            {comparison.issues.map((issue, index) => (
              <li key={`${issue.path}-${index}`} className="grid min-w-0 gap-0.5">
                <code className="text-xs break-all text-muted-foreground">{issue.path || t('comparison.rootPath')}</code>
                <span className="break-words">{issue.message}</span>
              </li>
            ))}
          </ul>
        </div>
      )}

      {comparison.summary.conflicts > 0 && (
        <p className="rounded-md border border-warning/40 bg-warning/10 p-3 text-sm">
          {t('comparison.conflictsNotice', { count: comparison.summary.conflicts })}
        </p>
      )}

      {groups.map((group) => (
        <section key={group.section} className="grid min-w-0 gap-2" aria-labelledby={`version-section-${group.section}`}>
          <h3 id={`version-section-${group.section}`} className="text-base font-semibold">
            {labels.section(group.section)}
          </h3>
          {group.relevant.map((item) => (
            <VersionItemCard
              key={item.id}
              item={item}
              resolution={choices[item.id] ?? 'keep_local'}
              onResolutionChange={(resolution) => choose(item.id, resolution)}
            />
          ))}
          {group.quiet.length > 0 && (
            <details className="min-w-0 rounded-lg border p-3">
              <summary className="cursor-pointer text-sm text-muted-foreground">
                {t('comparison.unchangedCount', { count: group.quiet.length })}
              </summary>
              <ul className="mt-2 grid gap-1 text-sm">
                {group.quiet.map((item) => (
                  <li key={item.id} className="flex min-w-0 flex-wrap items-center gap-2">
                    <span className="min-w-0 break-words">{item.label || item.key}</span>
                    <span className="text-xs text-muted-foreground">{labels.kind(item.kind)}</span>
                  </li>
                ))}
              </ul>
            </details>
          )}
        </section>
      ))}

      <div className="flex flex-col gap-3 border-t pt-4 sm:flex-row sm:items-center sm:justify-between">
        <p className="text-sm text-muted-foreground">
          {comparison.valid ? t('comparison.ready', { count: pending.length }) : t('comparison.cannotApply')}
        </p>
        <div className="flex flex-wrap gap-2">
          <Button variant="outline" onClick={onBack}>
            {t('comparison.cancel')}
          </Button>
          <Button disabled={!canApply} onClick={() => setConfirming(true)}>
            {comparison.installed ? t('comparison.update') : t('comparison.install')}
          </Button>
        </div>
      </div>

      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={t('apply.confirmTitle', { name: identity?.name ?? '', version: identity?.version ?? '' })}
        description={t('apply.confirmDescription', {
          keep: Object.values(resolutions).filter((value) => value === 'keep_local').length,
          accept: Object.values(resolutions).filter((value) => value === 'accept_package').length,
        })}
        confirmLabel={t('apply.confirm')}
        pending={applying}
        onConfirm={confirm}
      />
    </div>
  )
}

function SummaryList({ summary }: { summary: ComparisonSummary }) {
  const { t } = useTranslation('config-package-versioning')
  return (
    <dl className="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-6" aria-label={t('comparison.summaryLabel')}>
      {SUMMARY_ORDER.map((key) => (
        <div key={key} className="rounded-md border bg-muted/40 px-3 py-2">
          <dt className="text-xs text-muted-foreground">{t(`summary.${key}`)}</dt>
          <dd className="text-lg font-semibold tabular-nums">{summary[key]}</dd>
        </div>
      ))}
    </dl>
  )
}
