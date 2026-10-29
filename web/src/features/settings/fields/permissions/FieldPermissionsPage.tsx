import { ArrowLeft, Lock, ShieldCheck } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useParams } from 'react-router'
import { isApiError } from '@/api'
import { ConfirmDialog, UnsavedChangesGuard, notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { cn } from '@/lib/utils'
import { ListSkeleton } from '../../components/ListSkeleton'
import { RecordTypeSelect } from '../../components/RecordTypeSelect'
import { RequirePermission } from '../../components/RequirePermission'
import { SectionHeader } from '../../components/SectionHeader'
import { SETTINGS_MANAGE } from '../../lib/permissions'
import { isRecordTypeNotFound } from '../lib/errors'
import { DEFAULT_RECORD_TYPE } from '../lib/field-types'
import { useRecordTypeLabel } from '../lib/useRecordTypeLabel'
import {
  FIELD_ACCESS_LEVELS,
  useFieldPermissions,
  useSaveFieldPermissions,
  type FieldAccess,
  type FieldPermissionField,
  type FieldPermissionMatrix,
  type FieldPermissionRole,
} from './api'
import { applyToColumn, cellKey, changedCells, draftEntries, isConfigurable, matrixDraft, type MatrixDraft } from './matrix'

export function FieldPermissionsPage() {
  const { recordType = DEFAULT_RECORD_TYPE } = useParams()
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <FieldPermissionsSection key={recordType} recordType={recordType} />
    </RequirePermission>
  )
}

function FieldPermissionsSection({ recordType }: { recordType: string }) {
  const { t } = useTranslation('field-permissions')
  const { t: tf } = useTranslation('fields')
  const query = useFieldPermissions(recordType)
  const recordTypeLabel = useRecordTypeLabel(recordType)
  const unavailable = query.isError && isRecordTypeNotFound(query.error)

  return (
    <section className="grid min-w-0 grid-cols-1 gap-4">
      <div>
        <Button variant="ghost" size="sm" asChild>
          <Link to={`/settings/fields/${recordType}`}>
            <ArrowLeft />
            {t('back')}
          </Link>
        </Button>
      </div>
      <SectionHeader
        title={t('title')}
        description={t('description', { recordType: recordTypeLabel })}
        action={
          <RecordTypeSelect
            value={recordType}
            label={t('recordTypesLabel')}
            coreLabel={(type) => tf(`recordTypes.${type}`)}
            to={(type) => `/settings/fields/${type}/permissions`}
          />
        }
      />
      <Legend />
      {query.isPending ? (
        <ListSkeleton rows={6} />
      ) : unavailable ? (
        <EmptyState title={t('empty')} />
      ) : query.isError ? (
        <ErrorState onRetry={() => void query.refetch()} />
      ) : (
        <MatrixEditor
          key={query.data.updated_at ?? 'initial'}
          recordType={recordType}
          recordTypeLabel={recordTypeLabel}
          matrix={query.data}
          onReload={() => void query.refetch()}
        />
      )}
    </section>
  )
}

function Legend() {
  const { t } = useTranslation('field-permissions')
  return (
    <details className="rounded-lg border bg-muted/30 px-4 py-3 text-sm">
      <summary className="cursor-pointer font-medium">{t('legend.title')}</summary>
      <ul className="mt-2 grid list-disc gap-1 pl-5 text-muted-foreground">
        <li>{t('legend.hidden')}</li>
        <li>{t('legend.read_only')}</li>
        <li>{t('legend.editable')}</li>
        <li>{t('legend.locked')}</li>
        <li>{t('legend.admin')}</li>
      </ul>
    </details>
  )
}

interface MatrixEditorProps {
  recordType: string
  recordTypeLabel: string
  matrix: FieldPermissionMatrix
  onReload: () => void
}

function MatrixEditor({ recordType, recordTypeLabel, matrix, onReload }: MatrixEditorProps) {
  const { t } = useTranslation('field-permissions')
  const initial = useMemo(() => matrixDraft(matrix), [matrix])
  const [draft, setDraft] = useState<MatrixDraft>(initial)
  const [showInactive, setShowInactive] = useState(false)
  const [confirming, setConfirming] = useState(false)
  const [conflict, setConflict] = useState(false)
  const save = useSaveFieldPermissions(recordType)

  const fields = matrix.fields.filter((field) => showInactive || field.is_active)
  const roles = matrix.roles
  const configurableRoles = roles.filter((role) => role.is_configurable)
  const changed = changedCells(initial, draft)
  const dirty = changed.length > 0

  const setCell = (field: FieldPermissionField, role: FieldPermissionRole, access: FieldAccess) =>
    setDraft((current) => ({ ...current, [cellKey(field.id, role.id)]: access }))

  const submit = async () => {
    try {
      await save.mutateAsync({ entries: draftEntries(matrix, draft), version: matrix.updated_at })
      notifySuccess(t('saved'))
    } catch (error) {
      if (isApiError(error) && error.code === 'version_conflict') {
        setConflict(true)
        return
      }
      const title =
        isApiError(error) && error.code === 'field_permission_locked'
          ? t('errors.locked')
          : isApiError(error) && error.status === 422
            ? t('errors.invalid')
            : t('errors.generic')
      notifyError(error, { title })
    } finally {
      setConfirming(false)
    }
  }

  return (
    <div className="grid min-w-0 gap-4">
      <UnsavedChangesGuard when={dirty && !save.isPending} />
      {conflict && (
        <div
          role="alert"
          className="flex flex-col gap-3 rounded-lg border border-destructive/30 bg-destructive/5 p-4 text-sm sm:flex-row sm:items-center sm:justify-between"
        >
          <div className="grid gap-1">
            <p className="font-medium">{t('conflict.title')}</p>
            <p className="text-muted-foreground">{t('conflict.description')}</p>
          </div>
          <Button variant="outline" size="sm" onClick={onReload}>
            {t('conflict.reload')}
          </Button>
        </div>
      )}
      {configurableRoles.length === 0 && <p className="text-sm text-muted-foreground">{t('noRoles')}</p>}
      <label className="flex items-center gap-2 text-sm text-muted-foreground">
        <input
          type="checkbox"
          className="size-4 accent-primary"
          checked={showInactive}
          onChange={(event) => setShowInactive(event.target.checked)}
        />
        {t('showInactive')}
      </label>
      {fields.length === 0 ? (
        <EmptyState title={t('empty')} />
      ) : (
        <div className="relative max-w-full overflow-x-auto rounded-lg border">
          <table className="w-full min-w-max text-sm">
            <caption className="sr-only">{t('table.caption', { recordType: recordTypeLabel })}</caption>
            <thead className="bg-muted/50 text-muted-foreground">
              <tr>
                <th scope="col" className="sticky left-0 z-10 bg-muted px-3 py-2 text-left align-bottom font-medium">
                  {t('table.field')}
                </th>
                {roles.map((role) => (
                  <th key={role.id} scope="col" className="px-3 py-2 text-left align-bottom font-medium">
                    <div className="grid gap-1.5">
                      <span className="text-foreground">{role.name}</span>
                      {role.is_configurable ? (
                        <ColumnApply
                          role={role}
                          onApply={(access) => setDraft((current) => applyToColumn(current, fields, role.id, access))}
                        />
                      ) : (
                        <span className="inline-flex items-center gap-1 text-xs font-normal">
                          <Lock className="size-3" aria-hidden />
                          {t('table.admin')}
                        </span>
                      )}
                    </div>
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {fields.map((field) => (
                <tr key={field.id} className="border-t">
                  <th scope="row" className="sticky left-0 z-10 bg-background px-3 py-2 text-left font-normal">
                    <FieldLabel field={field} />
                  </th>
                  {roles.map((role) => (
                    <td key={role.id} className="px-3 py-2">
                      <AccessCell
                        field={field}
                        role={role}
                        value={draft[cellKey(field.id, role.id)] ?? 'editable'}
                        changed={changed.includes(cellKey(field.id, role.id))}
                        disabled={save.isPending}
                        onChange={(access) => setCell(field, role, access)}
                      />
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <div className="flex flex-col-reverse gap-2 sm:flex-row sm:items-center sm:justify-end">
        <p className="text-sm text-muted-foreground sm:mr-auto" aria-live="polite">
          {dirty ? t('pending', { count: changed.length }) : t('noChanges')}
        </p>
        <Button variant="outline" disabled={!dirty || save.isPending} onClick={() => setDraft(initial)}>
          {t('discard')}
        </Button>
        <Button disabled={!dirty || save.isPending || conflict} onClick={() => setConfirming(true)}>
          <ShieldCheck />
          {save.isPending ? t('saving') : t('save')}
        </Button>
      </div>
      <ConfirmDialog
        open={confirming}
        onOpenChange={setConfirming}
        title={t('confirm.title')}
        description={t('confirm.description', { count: changed.length })}
        confirmLabel={t('confirm.confirm')}
        pending={save.isPending}
        notifyOnError={false}
        onConfirm={submit}
      />
    </div>
  )
}

function FieldLabel({ field }: { field: FieldPermissionField }) {
  const { t } = useTranslation('field-permissions')
  return (
    <div className="grid max-w-56 gap-1">
      <span className="font-medium break-words text-foreground">{field.label}</span>
      <span className="flex flex-wrap gap-1">
        {field.is_system && <Badge variant="secondary">{t('table.system')}</Badge>}
        {!field.is_active && <Badge variant="outline">{t('table.inactive')}</Badge>}
        {field.is_sensitive ? (
          <Badge variant="warning">{t('table.sensitive')}</Badge>
        ) : (
          field.is_personal && <Badge variant="outline">{t('table.personal')}</Badge>
        )}
      </span>
    </div>
  )
}

function ColumnApply({ role, onApply }: { role: FieldPermissionRole; onApply: (access: FieldAccess) => void }) {
  const { t } = useTranslation('field-permissions')
  return (
    <select
      aria-label={t('table.applyToColumnLabel', { role: role.name })}
      title={t('table.applyToColumn')}
      className="h-8 rounded-md border border-input bg-background px-2 text-xs font-normal text-foreground"
      value=""
      onChange={(event) => {
        const access = event.target.value as FieldAccess
        if (access) onApply(access)
      }}
    >
      <option value="">{t('table.applyPlaceholder')}</option>
      {FIELD_ACCESS_LEVELS.map((access) => (
        <option key={access} value={access}>
          {t(`access.${access}`)}
        </option>
      ))}
    </select>
  )
}

interface AccessCellProps {
  field: FieldPermissionField
  role: FieldPermissionRole
  value: FieldAccess
  changed: boolean
  disabled: boolean
  onChange: (access: FieldAccess) => void
}

function AccessCell({ field, role, value, changed, disabled, onChange }: AccessCellProps) {
  const { t } = useTranslation('field-permissions')
  if (!role.is_configurable || !isConfigurable(field)) {
    return (
      <span
        className="inline-flex items-center gap-1 text-muted-foreground"
        title={role.is_configurable ? t('table.lockedHint') : t('table.admin')}
      >
        <Lock className="size-3" aria-hidden />
        {t('access.editable')}
        <span className="sr-only">
          ({role.is_configurable ? t('table.locked') : t('table.admin')})
        </span>
      </span>
    )
  }
  return (
    <select
      aria-label={`${t('table.cell', { field: field.label, role: role.name })}${changed ? ` (${t('table.changed')})` : ''}`}
      className={cn(
        'h-8 w-full min-w-36 rounded-md border border-input bg-background px-2 text-sm',
        value === 'hidden' && 'text-destructive',
        changed && 'ring-2 ring-primary/40',
      )}
      value={value}
      disabled={disabled}
      onChange={(event) => onChange(event.target.value as FieldAccess)}
    >
      {FIELD_ACCESS_LEVELS.map((access) => (
        <option key={access} value={access}>
          {t(`access.${access}`)}
        </option>
      ))}
    </select>
  )
}
