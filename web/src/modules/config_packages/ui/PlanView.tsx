import { ArrowLeft, CircleAlert } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import {
  PLAN_OUTDATED_CODE,
  useApplyPackage,
  type ConflictDecision,
  type Decisions,
  type PackagePlan,
  type PackageSource,
  type PlanItem,
  type PlanSummary,
} from '@/modules/config_packages/ui/api'
import { usePackageLabels } from '@/modules/config_packages/ui/lib/labels'
import { PlanItemCard } from '@/modules/config_packages/ui/PlanItemCard'

const SUMMARY_ORDER = ['create', 'update', 'unchanged', 'skip_conflict'] as const

interface PlanViewProps {
  plan: PackagePlan
  source: PackageSource
  notice: string | null
  replanning: boolean
  onBack: () => void
  onOutdated: () => Promise<void>
  onApplied: () => void
}

interface SectionGroup {
  section: string
  changed: PlanItem[]
  unchanged: PlanItem[]
}

function groupBySection(plan: PackagePlan): SectionGroup[] {
  const order = [...plan.sections, ...plan.items.map((item) => item.section).filter((section) => !plan.sections.includes(section))]
  return [...new Set(order)]
    .map((section) => {
      const items = plan.items.filter((item) => item.section === section)
      return {
        section,
        changed: items.filter((item) => item.action !== 'unchanged'),
        unchanged: items.filter((item) => item.action === 'unchanged'),
      }
    })
    .filter((group) => group.changed.length + group.unchanged.length > 0)
}

function isResolvableConflict(item: PlanItem): boolean {
  return item.action === 'skip_conflict' && item.conflict?.resolvable === true
}

export function PlanView({ plan, source, notice, replanning, onBack, onOutdated, onApplied }: PlanViewProps) {
  const { t } = useTranslation('config-packages')
  const labels = usePackageLabels()
  const apply = useApplyPackage()
  const [choices, setChoices] = useState<Decisions>({})
  const [confirming, setConfirming] = useState(false)
  const groups = useMemo(() => groupBySection(plan), [plan])

  const decisions = useMemo(
    () =>
      Object.fromEntries(
        plan.items.filter(isResolvableConflict).map((item) => [item.id, choices[item.id] ?? 'skip']),
      ) as Decisions,
    [plan, choices],
  )
  const forced = Object.values(decisions).filter((decision) => decision === 'apply').length
  const pendingChanges = plan.summary.create + plan.summary.update + forced
  const canApply = plan.valid && pendingChanges > 0 && !replanning

  const choose = (id: string, decision: ConflictDecision) => setChoices((current) => ({ ...current, [id]: decision }))

  const confirm = async () => {
    try {
      const result = await apply.mutateAsync({ source, fingerprint: plan.fingerprint, decisions })
      notifySuccess(t('apply.success', { name: result.install.name }), {
        description: t('apply.summary', {
          create: result.summary.create,
          update: result.summary.update,
          skipped: result.summary.skip_conflict,
        }),
      })
      onApplied()
    } catch (error) {
      if (isApiError(error) && error.code === PLAN_OUTDATED_CODE) {
        await onOutdated()
        return
      }
      throw error
    }
  }

  const packageName = plan.package?.name ?? plan.template_key ?? ''

  return (
    <div className="grid min-w-0 gap-4">
      <div>
        <Button variant="ghost" size="sm" onClick={onBack}>
          <ArrowLeft aria-hidden="true" />
          {t('plan.back')}
        </Button>
      </div>
      <Card>
        <CardHeader>
          <CardTitle className="break-words">{t('plan.title', { name: packageName })}</CardTitle>
          {plan.package?.description && (
            <CardDescription className="break-words">{plan.package.description}</CardDescription>
          )}
        </CardHeader>
        <CardContent className="grid gap-3">
          <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
            {plan.package?.author && <span>{t('install.author', { author: plan.package.author })}</span>}
            {plan.template_key && <span>{t('plan.template', { key: plan.template_key })}</span>}
            {plan.package?.locale && <span>{t('plan.locale', { locale: plan.package.locale })}</span>}
          </div>
          <SummaryList summary={plan.summary} label={t('plan.summaryLabel')} />
        </CardContent>
      </Card>

      <div role="status" aria-live="polite">
        {notice && (
          <p className="rounded-md border border-warning/40 bg-warning/10 p-3 text-sm">
            {replanning ? t('plan.replanning') : notice}
          </p>
        )}
      </div>

      {!plan.valid && (
        <div role="alert" className="grid gap-2 rounded-lg border border-destructive/30 bg-destructive/5 p-4">
          <p className="flex items-center gap-2 font-medium text-destructive">
            <CircleAlert className="size-4" aria-hidden="true" />
            {t('plan.invalidTitle')}
          </p>
          <p className="text-sm text-muted-foreground">{t('plan.invalidDescription')}</p>
          <ul className="grid gap-1.5 text-sm" aria-label={t('plan.issuesLabel')}>
            {plan.issues.map((issue, index) => (
              <li key={`${issue.path}-${index}`} className="grid min-w-0 gap-0.5">
                <code className="text-xs break-all text-muted-foreground">{issue.path || t('plan.rootPath')}</code>
                <span className="break-words">{issue.message}</span>
              </li>
            ))}
          </ul>
        </div>
      )}

      {plan.valid && groups.length === 0 && (
        <p className="text-sm text-muted-foreground">{t('plan.emptyPlan')}</p>
      )}

      {groups.map((group) => (
        <section key={group.section} className="grid min-w-0 gap-2" aria-labelledby={`plan-section-${group.section}`}>
          <h3 id={`plan-section-${group.section}`} className="text-base font-semibold">
            {labels.section(group.section)}
          </h3>
          {group.changed.length > 0 && (
            <div className="grid min-w-0 gap-2">
              {group.changed.map((item) => (
                <PlanItemCard
                  key={item.id}
                  item={item}
                  decision={choices[item.id] ?? 'skip'}
                  onDecisionChange={(decision) => choose(item.id, decision)}
                />
              ))}
            </div>
          )}
          {group.unchanged.length > 0 && (
            <details className="min-w-0 rounded-lg border p-3">
              <summary className="cursor-pointer text-sm text-muted-foreground">
                {t('plan.unchangedCount', { count: group.unchanged.length })}
              </summary>
              <ul className="mt-2 grid gap-1 text-sm">
                {group.unchanged.map((item) => (
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
          {!plan.valid
            ? t('plan.cannotApplyInvalid')
            : pendingChanges === 0
              ? t('plan.nothingToApply')
              : t('plan.readyToApply', { count: pendingChanges })}
        </p>
        <div className="flex flex-wrap gap-2">
          <Button variant="outline" onClick={onBack}>
            {t('plan.cancel')}
          </Button>
          <Button disabled={!canApply} onClick={() => setConfirming(true)}>
            {t('plan.apply')}
          </Button>
        </div>
      </div>

      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={t('apply.confirmTitle', { name: packageName })}
        description={t('apply.confirmDescription', {
          create: plan.summary.create,
          update: plan.summary.update + forced,
        })}
        confirmLabel={t('apply.confirm')}
        pending={apply.isPending}
        onConfirm={confirm}
      />
    </div>
  )
}

function SummaryList({ summary, label }: { summary: PlanSummary; label: string }) {
  const { t } = useTranslation('config-packages')
  return (
    <dl className="grid grid-cols-2 gap-2 sm:grid-cols-4" aria-label={label}>
      {SUMMARY_ORDER.map((key) => (
        <div key={key} className="rounded-md border bg-muted/40 px-3 py-2">
          <dt className="text-xs text-muted-foreground">{t(`summary.${key}`)}</dt>
          <dd className="text-lg font-semibold tabular-nums">{summary[key]}</dd>
        </div>
      ))}
    </dl>
  )
}
