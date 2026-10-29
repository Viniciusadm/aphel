import { ArrowDown, ArrowUp, Pencil, Plus, ShieldCheck, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, PageLoader } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { toast } from '@/components/ui/toast-store'
import { usePipelines } from '@/modules/pipelines/api'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { SectionHeader } from '@/features/settings/components/SectionHeader'
import { approvalsApi, useApprovalPolicies, usePolicyMutation } from '../api'
import { APPROVALS_MANAGE_PERMISSION } from '../constants'
import { approvalErrorKey } from '../errors'
import { useRecordTypeLabel } from '../lib/labels'
import type { ApprovalPolicy } from '../types'
import { PolicyEditorDialog } from './PolicyEditorDialog'

interface EditorState {
  policy: ApprovalPolicy | null
}

export function ApprovalPoliciesPage() {
  return (
    <RequirePermission permission={APPROVALS_MANAGE_PERMISSION}>
      <ApprovalPolicies />
    </RequirePermission>
  )
}

function ApprovalPolicies() {
  const { t } = useTranslation('approvals')
  const policies = useApprovalPolicies()
  const [editor, setEditor] = useState<EditorState | null>(null)
  const [removing, setRemoving] = useState<ApprovalPolicy | null>(null)
  const notifyFailure = (error: unknown) => toast({ title: t(approvalErrorKey(error)), variant: 'destructive' })

  const reorder = usePolicyMutation((ids: string[]) => approvalsApi.reorderPolicies(ids))
  const toggle = usePolicyMutation((policy: ApprovalPolicy) =>
    approvalsApi.updatePolicy(policy.id, { is_active: !policy.is_active }, policy.updated_at),
  )
  const remove = usePolicyMutation((policy: ApprovalPolicy) => approvalsApi.removePolicy(policy.id))

  const ordered = [...(policies.data ?? [])].sort((left, right) => left.position - right.position)

  const move = (index: number, offset: number) => {
    const target = index + offset
    if (target < 0 || target >= ordered.length) return
    const ids = ordered.map((policy) => policy.id)
    const [item] = ids.splice(index, 1)
    ids.splice(target, 0, item)
    reorder.mutate(ids, { onError: notifyFailure })
  }

  const newButton = (
    <Button onClick={() => setEditor({ policy: null })}>
      <Plus aria-hidden="true" />
      {t('settings.new')}
    </Button>
  )

  let body
  if (policies.isPending) {
    body = <PageLoader />
  } else if (policies.isError) {
    body = <ErrorState onRetry={() => void policies.refetch()} />
  } else if (ordered.length === 0) {
    body = (
      <EmptyState
        icon={ShieldCheck}
        title={t('settings.emptyTitle')}
        description={t('settings.emptyDescription')}
        action={newButton}
      />
    )
  } else {
    body = (
      <ul className="grid gap-2" aria-label={t('settings.listLabel')}>
        {ordered.map((policy, index) => (
          <PolicyRow
            key={policy.id}
            policy={policy}
            first={index === 0}
            last={index === ordered.length - 1}
            busy={reorder.isPending || toggle.isPending}
            onMoveUp={() => move(index, -1)}
            onMoveDown={() => move(index, 1)}
            onToggle={() => toggle.mutate(policy, { onError: notifyFailure })}
            onEdit={() => setEditor({ policy })}
            onRemove={() => setRemoving(policy)}
          />
        ))}
      </ul>
    )
  }

  return (
    <section className="flex flex-col gap-6">
      <SectionHeader
        title={t('settings.title')}
        description={t('settings.description')}
        action={ordered.length > 0 ? newButton : undefined}
      />
      {body}
      <PolicyEditorDialog
        open={editor !== null}
        policy={editor?.policy ?? null}
        onOpenChange={(open) => {
          if (!open) setEditor(null)
        }}
      />
      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => {
          if (!open) setRemoving(null)
        }}
        title={t('settings.removeTitle')}
        description={t('settings.removeDescription', {
          name: removing?.name ?? '',
        })}
        confirmLabel={t('settings.remove')}
        variant="destructive"
        pending={remove.isPending}
        onConfirm={async () => {
          if (!removing) return
          await remove.mutateAsync(removing)
          toast({ title: t('settings.removed'), variant: 'success' })
          setRemoving(null)
        }}
      />
    </section>
  )
}

interface PolicyRowProps {
  policy: ApprovalPolicy
  first: boolean
  last: boolean
  busy: boolean
  onMoveUp: () => void
  onMoveDown: () => void
  onToggle: () => void
  onEdit: () => void
  onRemove: () => void
}

function PolicyRow({ policy, first, last, busy, onMoveUp, onMoveDown, onToggle, onEdit, onRemove }: PolicyRowProps) {
  const { t } = useTranslation('approvals')
  const recordTypeLabel = useRecordTypeLabel()
  const pipelines = usePipelines()
  const stage = policy.target_id
    ? pipelines.data?.flatMap((pipeline) => pipeline.stages).find((item) => item.id === policy.target_id)
    : undefined

  return (
    <li className="flex flex-wrap items-center gap-3 rounded-md border bg-card px-3 py-2">
      <div className="flex flex-col">
        <Button
          type="button"
          size="icon"
          variant="ghost"
          className="size-7"
          aria-label={t('settings.moveUp', { name: policy.name })}
          disabled={first || busy}
          onClick={onMoveUp}
        >
          <ArrowUp aria-hidden="true" />
        </Button>
        <Button
          type="button"
          size="icon"
          variant="ghost"
          className="size-7"
          aria-label={t('settings.moveDown', { name: policy.name })}
          disabled={last || busy}
          onClick={onMoveDown}
        >
          <ArrowDown aria-hidden="true" />
        </Button>
      </div>
      <div className="grid min-w-0 flex-1 gap-0.5">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-medium">{policy.name}</span>
          <span className="font-mono text-xs text-muted-foreground">{policy.key}</span>
          {!policy.is_active && <Badge variant="secondary">{t('settings.inactive')}</Badge>}
        </div>
        <span className="text-xs text-muted-foreground">
          {recordTypeLabel(policy.record_type)} · {t(`moments.${policy.moment}`)}
          {stage && ` · ${stage.name}`} · {t(`conditionKinds.${policy.condition_kind}`)} ·{' '}
          {t('settings.stepsCount', { count: policy.steps.length })}
        </span>
      </div>
      <div className="flex items-center gap-1">
        <label className="mr-2 flex items-center gap-2 text-xs text-muted-foreground">
          <Checkbox
            checked={policy.is_active}
            disabled={busy}
            aria-label={t('settings.toggle', { name: policy.name })}
            onCheckedChange={onToggle}
          />
          {t('settings.active')}
        </label>
        <Button
          type="button"
          size="icon"
          variant="ghost"
          aria-label={t('settings.edit', { name: policy.name })}
          onClick={onEdit}
        >
          <Pencil aria-hidden="true" />
        </Button>
        <Button
          type="button"
          size="icon"
          variant="ghost"
          aria-label={t('settings.removeLabel', { name: policy.name })}
          onClick={onRemove}
        >
          <Trash2 aria-hidden="true" />
        </Button>
      </div>
    </li>
  )
}
