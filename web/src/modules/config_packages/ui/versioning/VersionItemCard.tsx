import { ArrowRight } from 'lucide-react'
import { useId } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge, type BadgeProps } from '@/components/ui/badge'
import type { PlanChange } from '@/modules/config_packages/ui/api'
import { useValueDescriber } from '@/modules/config_packages/ui/lib/describe-value'
import { usePackageLabels } from '@/modules/config_packages/ui/lib/labels'
import { ActionBadge } from '@/modules/config_packages/ui/PlanItemCard'
import type { PackageStatus, Resolution, VersionItem } from '@/modules/config_packages/ui/versioning/types'

const STATUS_VARIANT: Record<PackageStatus, BadgeProps['variant']> = {
  added: 'success',
  changed: 'default',
  removed: 'secondary',
  unchanged: 'outline',
}

const RESOLUTIONS: readonly Resolution[] = ['keep_local', 'accept_package']

interface VersionItemCardProps {
  item: VersionItem
  resolution: Resolution
  onResolutionChange: (resolution: Resolution) => void
}

export function VersionItemCard({ item, resolution, onResolutionChange }: VersionItemCardProps) {
  const { t } = useTranslation('config-package-versioning')
  const labels = usePackageLabels()
  const title = item.label || item.key
  const planChanges = item.plan && item.plan.action !== 'unchanged' ? item.plan.changes : []

  return (
    <article
      className={
        item.conflict
          ? 'grid min-w-0 gap-3 rounded-lg border border-warning/60 bg-card p-3 sm:p-4'
          : 'grid min-w-0 gap-3 rounded-lg border bg-card p-3 sm:p-4'
      }
      aria-label={title}
    >
      <header className="flex min-w-0 flex-wrap items-center gap-2">
        <h4 className="min-w-0 font-medium break-words">{title}</h4>
        <Badge variant="secondary">{labels.kind(item.kind)}</Badge>
        <Badge variant={STATUS_VARIANT[item.package_status]}>{t(`status.${item.package_status}`)}</Badge>
        {item.locally_modified && <Badge variant="outline">{t('item.locallyModified')}</Badge>}
        {item.conflict && <Badge variant="warning">{t('item.conflict')}</Badge>}
        {item.plan && <ActionBadge action={item.plan.action} />}
        <code className="min-w-0 text-xs break-all text-muted-foreground">{item.key}</code>
      </header>
      {item.package_status === 'removed' && <p className="text-sm text-muted-foreground">{t('item.removedHint')}</p>}
      {item.package_changes.length > 0 && item.package_status !== 'removed' && (
        <ChangeList title={t('item.packageChanges')} changes={item.package_changes} />
      )}
      {planChanges.length > 0 && (
        <ChangeList title={t('item.localChanges')} changes={planChanges} creating={item.plan?.action === 'create'} />
      )}
      {item.plan?.conflict && <p className="text-sm break-words text-muted-foreground">{item.plan.conflict.message}</p>}
      {item.needs_resolution && (
        <ResolutionChoice title={title} conflict={item.conflict} value={resolution} onChange={onResolutionChange} />
      )}
    </article>
  )
}

function ChangeList({ title, changes, creating = false }: { title: string; changes: PlanChange[]; creating?: boolean }) {
  const labels = usePackageLabels()
  const describe = useValueDescriber()
  const { t } = useTranslation('config-package-versioning')
  return (
    <div className="grid min-w-0 gap-1">
      <span className="text-xs font-medium tracking-wide text-muted-foreground uppercase">{title}</span>
      <ul className="grid min-w-0 gap-1 text-sm">
        {changes.map((change) => {
          const current: unknown = change.current
          const proposed: unknown = change.proposed
          const showCurrent = !creating && current !== null && current !== undefined
          return (
            <li key={change.property} className="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-1">
              <span className="font-medium text-muted-foreground">{labels.property(change.property)}</span>
              {showCurrent && (
                <>
                  <span className="min-w-0 break-words text-muted-foreground line-through">
                    <span className="sr-only">{t('item.before')} </span>
                    {describe(current)}
                  </span>
                  <ArrowRight className="size-3.5 shrink-0 self-center text-muted-foreground" aria-hidden="true" />
                </>
              )}
              <span className="min-w-0 break-words">
                <span className="sr-only">{t('item.after')} </span>
                {describe(proposed)}
              </span>
            </li>
          )
        })}
      </ul>
    </div>
  )
}

interface ResolutionChoiceProps {
  title: string
  conflict: boolean
  value: Resolution
  onChange: (resolution: Resolution) => void
}

function ResolutionChoice({ title, conflict, value, onChange }: ResolutionChoiceProps) {
  const { t } = useTranslation('config-package-versioning')
  const name = useId()
  return (
    <fieldset className="grid gap-1.5 rounded-md border border-warning/40 bg-warning/10 p-3 text-sm">
      <legend className="sr-only">{t('item.resolutionLegend', { name: title })}</legend>
      <p className="font-medium">{conflict ? t('item.conflictHint') : t('item.planConflictHint')}</p>
      {RESOLUTIONS.map((option) => (
        <label key={option} className="flex items-center gap-2">
          <input
            type="radio"
            name={name}
            value={option}
            checked={value === option}
            className="size-4 accent-primary"
            onChange={() => onChange(option)}
          />
          {t(`resolutions.${option}`)}
        </label>
      ))}
    </fieldset>
  )
}
