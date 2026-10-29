import { Eye, EyeOff, History, RotateCcw } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { useOrganizationUsers } from '@/features/records/api'
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import type { FieldSchema } from '@/features/records/types'
import { useAutomationVersions } from '../api'
import { FlowEditor } from '../editor/FlowEditor'
import { EMPTY_REPORT } from '../lib/errors'
import { definitionToFlow } from '../lib/flow'
import { useAutomationLabels } from '../lib/labels'
import type { Automation, AutomationCatalog, AutomationVersion } from '../types'

export interface VersionsPanelProps {
  automation: Automation
  catalog: AutomationCatalog
  schema: FieldSchema | undefined
  readOnly?: boolean
  onRestore: (version: AutomationVersion) => void
}

export function VersionsPanel({ automation, catalog, schema, readOnly, onRestore }: VersionsPanelProps) {
  const { t } = useTranslation('automations')
  const versions = useAutomationVersions(automation.id)
  const users = useOrganizationUsers()
  const formatting = useRecordFormatting()
  const labels = useAutomationLabels(catalog)
  const [open, setOpen] = useState<number | null>(null)
  const [restoring, setRestoring] = useState<AutomationVersion | null>(null)
  const sorted = useMemo(() => [...(versions.data ?? [])].sort((left, right) => right.version - left.version), [versions.data])

  if (versions.isPending) return <Skeleton className="h-24 w-full" />
  if (versions.isError) return <ErrorState onRetry={() => void versions.refetch()} />
  if (sorted.length === 0) {
    return <EmptyState icon={History} title={t('versions.emptyTitle')} description={t('versions.emptyDescription')} />
  }

  return (
    <>
      <ol className="grid gap-3" aria-label={t('versions.listLabel')}>
        {sorted.map((version) => {
          const publisher = users.data?.find((user) => user.id === version.published_by)?.display_name
          const expanded = open === version.version
          return (
            <li key={version.id} className="grid gap-3 rounded-lg border bg-card p-4">
              <div className="flex flex-wrap items-start justify-between gap-2">
                <div className="grid min-w-0 gap-0.5">
                  <span className="flex flex-wrap items-center gap-2 font-medium">
                    {t('versions.version', { version: version.version })}
                    {version.version === automation.version && <Badge variant="success">{t('versions.current')}</Badge>}
                  </span>
                  <span className="text-xs text-muted-foreground">
                    {t('versions.publishedBy', {
                      date: formatDateTime(version.published_at, formatting),
                      user: publisher ?? t('versions.unknownUser'),
                    })}
                  </span>
                  <span className="truncate text-xs text-muted-foreground">
                    {version.name} · {labels.triggerLabel(version.trigger.type)} ·{' '}
                    {t('versions.steps', { count: version.definition.steps.length })}
                  </span>
                </div>
                <div className="flex gap-1">
                  <Button type="button" variant="ghost" size="sm" aria-expanded={expanded} onClick={() => setOpen(expanded ? null : version.version)}>
                    {expanded ? <EyeOff aria-hidden="true" /> : <Eye aria-hidden="true" />}
                    {expanded ? t('versions.hide') : t('versions.view')}
                  </Button>
                  {!readOnly && (
                    <Button type="button" variant="outline" size="sm" onClick={() => setRestoring(version)}>
                      <RotateCcw aria-hidden="true" />
                      {t('versions.restore')}
                    </Button>
                  )}
                </div>
              </div>
              {expanded && (
                <FlowEditor
                  recordType={version.record_type}
                  trigger={version.trigger}
                  nodes={definitionToFlow(version.definition)}
                  catalog={catalog}
                  schema={schema}
                  report={EMPTY_REPORT}
                  selection={null}
                  readOnly
                  onSelect={() => undefined}
                  onInsert={() => undefined}
                  onRemoveJump={() => undefined}
                />
              )}
            </li>
          )
        })}
      </ol>
      <ConfirmDialog
        open={restoring !== null}
        onOpenChange={(value) => !value && setRestoring(null)}
        title={t('versions.restoreTitle', { version: restoring?.version ?? 0 })}
        description={t('versions.restoreDescription')}
        confirmLabel={t('versions.restore')}
        onConfirm={() => {
          if (restoring) onRestore(restoring)
          setRestoring(null)
        }}
      />
    </>
  )
}
