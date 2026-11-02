import { ArrowDown, ArrowUp, ListChecks, Pencil, Plus, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useParams } from 'react-router'
import { ConfirmDialog, PageLoader } from '@/components/feedback'
import { notifyError, notifySuccess } from '@/components/feedback/notify'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { useFieldSchema } from '@/features/records/api'
import { RecordTypeSelect } from '../components/RecordTypeSelect'
import { RequirePermission } from '../components/RequirePermission'
import { DEFAULT_RECORD_TYPE } from '../fields/lib/field-types'
import { SETTINGS_MANAGE } from '../lib/permissions'
import { useInvalidateValidationRules, useValidationRules, validationRulesApi } from './api'
import { VALIDATION_RULES_PATH } from './routes'
import type { ValidationRule } from './types'
import { ValidationRuleDialog } from './ValidationRuleDialog'
import { useIsModuleActive } from '@/kernel/modules/registry'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function ValidationRulesPage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <ValidationRulesSettings />
    </RequirePermission>
  )
}

interface EditorState {
  rule: ValidationRule | null
}

function ValidationRulesSettings() {
  const { t } = useTranslation('validation-rules')
  const { t: tFields } = useTranslation('fields')
  const quotesActive = useIsModuleActive('quotes')
  const { recordType = DEFAULT_RECORD_TYPE } = useParams()
  const rules = useValidationRules(recordType)
  const schema = useFieldSchema(recordType)
  const invalidate = useInvalidateValidationRules()
  const [editor, setEditor] = useState<EditorState | null>(null)
  const [removing, setRemoving] = useState<ValidationRule | null>(null)
  const [busy, setBusy] = useState(false)
  const fieldLabel = (key: string) => schema.data?.fields.find((field) => field.key === key)?.label ?? key

  const run = async (task: () => Promise<unknown>, message: string) => {
    setBusy(true)
    try {
      await task()
      await invalidate()
      notifySuccess(message)
    } catch (error) {
      notifyError(error)
    } finally {
      setBusy(false)
    }
  }

  const toggle = (rule: ValidationRule) =>
    run(
      () => validationRulesApi.update(rule.id, { is_active: !rule.is_active }, rule.updated_at),
      rule.is_active ? t('toasts.deactivated') : t('toasts.activated'),
    )

  const move = (index: number, offset: number) => {
    const list = rules.data ?? []
    const target = index + offset
    if (target < 0 || target >= list.length) return
    const ids = list.map((rule) => rule.id)
    ;[ids[index], ids[target]] = [ids[target], ids[index]]
    void run(() => validationRulesApi.reorder(recordType, ids), t('toasts.reordered'))
  }

  return (
    <Page>
      <PageHeader title={t('title')} description={t('description')} />
      <div className="flex flex-wrap items-center justify-between gap-3">
        <RecordTypeSelect
          value={recordType}
          label={t('recordType')}
          coreLabel={(type) => tFields(`recordTypes.${type}`)}
          to={(type) => `${VALIDATION_RULES_PATH}/${type}`}
          extraTypes={quotesActive ? [{ value: 'quotes.quote', label: tFields('recordTypes.quote') }] : []}
        />
        <Button onClick={() => setEditor({ rule: null })}>
          <Plus aria-hidden="true" />
          {t('new')}
        </Button>
      </div>
      <p className="text-sm text-muted-foreground">{t('help')}</p>
      {rules.isPending ? (
        <PageLoader />
      ) : rules.isError ? (
        <ErrorState onRetry={() => void rules.refetch()} />
      ) : rules.data.length === 0 ? (
        <EmptyState icon={ListChecks} title={t('empty.title')} description={t('empty.description')} />
      ) : (
        <ul className="grid gap-2" aria-label={t('list.label')}>
          {rules.data.map((rule, index) => (
            <li key={rule.id} className="grid min-w-0 gap-2 rounded-md border bg-card p-3">
              <div className="flex min-w-0 flex-wrap items-start justify-between gap-2">
                <div className="grid min-w-0 gap-1">
                  <div className="flex min-w-0 flex-wrap items-center gap-2">
                    <span className="font-medium break-words">{rule.name}</span>
                    <Badge variant="secondary">{t(`timing.${rule.applies_on}`)}</Badge>
                    {!rule.is_active && <Badge variant="outline">{t('list.inactive')}</Badge>}
                    {rule.field && (
                      <span className="text-xs text-muted-foreground">
                        {t('list.highlight', { field: fieldLabel(rule.field) })}
                      </span>
                    )}
                  </div>
                  <p className="text-sm break-words text-muted-foreground">{rule.message}</p>
                </div>
                <div className="flex shrink-0 items-center gap-1">
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={busy}
                    onClick={() => void toggle(rule)}
                  >
                    {rule.is_active ? t('list.deactivate') : t('list.activate')}
                  </Button>
                  <Button
                    size="icon"
                    variant="ghost"
                    disabled={busy || index === 0}
                    aria-label={t('list.moveUp', { name: rule.name })}
                    onClick={() => move(index, -1)}
                  >
                    <ArrowUp aria-hidden="true" />
                  </Button>
                  <Button
                    size="icon"
                    variant="ghost"
                    disabled={busy || index === rules.data.length - 1}
                    aria-label={t('list.moveDown', { name: rule.name })}
                    onClick={() => move(index, 1)}
                  >
                    <ArrowDown aria-hidden="true" />
                  </Button>
                  <Button
                    size="icon"
                    variant="ghost"
                    aria-label={t('list.edit', { name: rule.name })}
                    onClick={() => setEditor({ rule })}
                  >
                    <Pencil aria-hidden="true" />
                  </Button>
                  <Button
                    size="icon"
                    variant="ghost"
                    className="text-destructive"
                    aria-label={t('list.remove', { name: rule.name })}
                    onClick={() => setRemoving(rule)}
                  >
                    <Trash2 aria-hidden="true" />
                  </Button>
                </div>
              </div>
              <code className="block rounded bg-muted px-2 py-1 font-mono text-xs break-all">{rule.expression}</code>
            </li>
          ))}
        </ul>
      )}
      <ValidationRuleDialog
        open={editor !== null}
        recordType={recordType}
        rule={editor?.rule ?? null}
        onOpenChange={(open) => {
          if (!open) setEditor(null)
        }}
        onSaved={(created) => {
          setEditor(null)
          void invalidate()
          notifySuccess(created ? t('toasts.created') : t('toasts.updated'))
        }}
      />
      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => {
          if (!open) setRemoving(null)
        }}
        title={t('removeDialog.title')}
        description={t('removeDialog.description', { name: removing?.name ?? '' })}
        confirmLabel={t('removeDialog.confirm')}
        variant="destructive"
        pending={busy}
        onConfirm={async () => {
          const rule = removing
          if (!rule) return
          await run(() => validationRulesApi.remove(rule.id), t('toasts.removed'))
          setRemoving(null)
        }}
      />
    </Page>
  )
}
