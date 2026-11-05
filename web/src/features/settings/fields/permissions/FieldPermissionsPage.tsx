import { Lock, ShieldCheck } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useParams } from 'react-router'
import { isApiError } from '@/api'
import { ConfirmDialog, UnsavedChangesGuard, notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { ListSkeleton } from '../../components/ListSkeleton'
import { RecordTypeSelect } from '../../components/RecordTypeSelect'
import { RequirePermission } from '../../components/RequirePermission'
import { SETTINGS_MANAGE } from '../../lib/permissions'
import { isRecordTypeNotFound } from '../lib/errors'
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
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { NativeSelect } from '@/components/ui/native-select'
import { Checkbox } from '@/components/ui/checkbox'
import { Page } from '@/components/ui/page'

export function FieldPermissionsPage() {
  const { recordType = '' } = useParams()
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <FieldPermissionsSection key={recordType} recordType={recordType} />
    </RequirePermission>
  )
}

function FieldPermissionsSection({ recordType }: { recordType: string }) {
  const { t } = useTranslation('field-permissions')
  const query = useFieldPermissions(recordType)
  const recordTypeLabel = useRecordTypeLabel(recordType)
  const unavailable = query.isError && isRecordTypeNotFound(query.error)

  return (
    <Page>
      <PageHeader
        back={{ to: `/settings/fields/${recordType}`, label: t('back') }}
        title={t('title')}
        description={t('description', { recordType: recordTypeLabel })}
        actions={
          <RecordTypeSelect
            value={recordType}
            label={t('recordTypesLabel')}
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
    </Page>
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
        <Checkbox checked={showInactive} onCheckedChange={(checked) => setShowInactive(checked === true)} />
        {t('showInactive')}
      </label>
      {fields.length === 0 ? (
        <EmptyState title={t('empty')} />
      ) : (
        <Table>
          <caption className="sr-only">{t('table.caption', { recordType: recordTypeLabel })}</caption>
          <TableHeader>
            <TableRow>
              <TableHead scope="col" className="sticky left-0 z-10 bg-muted text-left align-bottom">
                {t('table.field')}
              </TableHead>
              {roles.map((role) => (
                <TableHead key={role.id} scope="col" className="text-left align-bottom">
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
                </TableHead>
              ))}
            </TableRow>
          </TableHeader>
          <TableBody>
            {fields.map((field) => (
              <TableRow key={field.id}>
                <TableHead scope="row" className="sticky left-0 z-10 bg-background text-left font-normal">
                  <FieldLabel field={field} />
                </TableHead>
                {roles.map((role) => (
                  <TableCell key={role.id}>
                    <AccessCell
                      field={field}
                      role={role}
                      value={draft[cellKey(field.id, role.id)] ?? 'editable'}
                      changed={changed.includes(cellKey(field.id, role.id))}
                      disabled={save.isPending}
                      onChange={(access) => setCell(field, role, access)}
                    />
                  </TableCell>
                ))}
              </TableRow>
            ))}
          </TableBody>
        </Table>
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
    <NativeSelect
      aria-label={t('table.applyToColumnLabel', { role: role.name })}
      title={t('table.applyToColumn')}
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
    </NativeSelect>
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
    <NativeSelect
      aria-label={`${t('table.cell', { field: field.label, role: role.name })}${changed ? ` (${t('table.changed')})` : ''}`}
      value={value}
      disabled={disabled}
      onChange={(event) => onChange(event.target.value as FieldAccess)}
    >
      {FIELD_ACCESS_LEVELS.map((access) => (
        <option key={access} value={access}>
          {t(`access.${access}`)}
        </option>
      ))}
    </NativeSelect>
  )
}
