import { NativeSelect } from '@/components/ui/native-select'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Eye, Loader2, RotateCcw } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useParams, useSearchParams } from 'react-router'
import { isApiError } from '@/api'
import { ConfirmDialog, UnsavedChangesGuard } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { ErrorState } from '@/components/ui/error-state'
import { toast } from '@/components/ui/toast-store'
import { useFieldSchema } from '@/features/records/api'

import { fieldViolations } from '@/features/records/fields'
import { RecordForm } from '@/features/records/form/RecordForm'
import { formLayoutKeys, formLayoutsApi, savedLayout, useFormLayout } from '@/features/records/form/layout-api'
import { LAYOUT_CONTEXTS, layoutFields, layoutKeys, type FormLayout, type LayoutContext } from '@/features/records/form/lib/layout'
import type { FieldSchema } from '@/features/records/types'
import { ListSkeleton } from '@/features/settings/components/ListSkeleton'
import { RecordTypeSelect } from '@/features/settings/components/RecordTypeSelect'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { DEFAULT_RECORD_TYPE, isKnownRecordType } from '@/features/settings/fields/lib/field-types'
import { SETTINGS_MANAGE } from '@/features/settings/lib/permissions'
import { FieldRulesDialog } from '@/modules/record_layouts/ui/components/FieldRulesDialog'
import { LayoutCanvas } from '@/modules/record_layouts/ui/components/LayoutCanvas'
import { fieldAtPath, layoutCandidates, layoutsEqual, normalizeForSave, setFieldRules, startingLayout } from '@/modules/record_layouts/ui/lib/draft'
import { PageHeader } from '@/components/ui/page-header'
import { Alert } from '@/components/ui/alert'
import { Page } from '@/components/ui/page'

function parseContext(value: string | null): LayoutContext {
  return LAYOUT_CONTEXTS.includes(value as LayoutContext) ? (value as LayoutContext) : 'create'
}

export function LayoutsPage() {
  const { recordType = DEFAULT_RECORD_TYPE } = useParams()
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <LayoutsSection key={recordType} recordType={recordType} />
    </RequirePermission>
  )
}

function LayoutsSection({ recordType }: { recordType: string }) {
  const { t } = useTranslation('layouts')
  const [params, setParams] = useSearchParams()
  const context = parseContext(params.get('context'))
  const schema = useFieldSchema(recordType)
  const layout = useFormLayout(recordType, context)

  return (
    <Page>
<PageHeader
        title={t('title')}
        description={t('description')}
        actions={
          <div className="flex flex-wrap gap-2">
            <RecordTypeSelect
              value={recordType}
              label={t('recordTypes.label')}
              coreLabel={(type) => t(`recordTypes.${type}`)}
              to={(type) => `/settings/layouts/${type}?context=${context}`}
            />
            <NativeSelect value={context}
              onValueChange={(next) => setParams({ context: parseContext(next) }, { replace: true })} aria-label={t('contexts.label')} className="w-full sm:w-40">
                {LAYOUT_CONTEXTS.map((item) => (
                  <option key={item} value={item}>
                    {t(`contexts.${item}`)}
                  </option>
                ))}
              </NativeSelect>
          </div>
        }
      />
      <p className="text-sm text-muted-foreground">{t(`contexts.${context}Hint`)}</p>
      {schema.isPending || layout.isPending ? (
        <ListSkeleton />
      ) : schema.isError || layout.isError ? (
        <ErrorState
          title={isKnownRecordType(recordType) ? t('errors.load') : t('errors.unavailable')}
          onRetry={() => {
            void schema.refetch()
            void layout.refetch()
          }}
        />
      ) : (
        <LayoutWorkspace
          key={`${context}:${layout.dataUpdatedAt}`}
          recordType={recordType}
          context={context}
          schema={schema.data}
          saved={savedLayout(layout.data)}
        />
      )}
    </Page>
  )
}

interface LayoutWorkspaceProps {
  recordType: string
  context: LayoutContext
  schema: FieldSchema
  saved: FormLayout | null
}

function LayoutWorkspace({ recordType, context, schema, saved }: LayoutWorkspaceProps) {
  const { t } = useTranslation('layouts')
  const queryClient = useQueryClient()
  const candidates = useMemo(() => layoutCandidates(schema.fields, context), [schema.fields, context])
  const initial = useMemo(() => saved ?? startingLayout(schema.fields, context), [saved, schema.fields, context])
  const [draft, setDraft] = useState<FormLayout>(initial)
  const [editingRules, setEditingRules] = useState<string | null>(null)
  const [previewing, setPreviewing] = useState(false)
  const [confirmingReset, setConfirmingReset] = useState(false)
  const [problems, setProblems] = useState<string[]>([])
  const dirty = !layoutsEqual(draft, initial)

  const invalidate = () => queryClient.invalidateQueries({ queryKey: formLayoutKeys.recordType(recordType) })
  const save = useMutation({
    mutationFn: (layout: FormLayout) => formLayoutsApi.save(recordType, context, layout),
  })

  const describeProblems = (error: unknown): string[] => {
    if (!isApiError(error)) return []
    return fieldViolations(error.details).map((violation) => {
      const key = fieldAtPath(draft, violation.field)
      const label = key ? (schema.fields.find((field) => field.key === key)?.label ?? key) : violation.field
      return t('errors.field', { field: label, message: violation.message })
    })
  }

  const submit = async () => {
    setProblems([])
    try {
      const saved = await save.mutateAsync(normalizeForSave(draft, context))
      queryClient.setQueryData(formLayoutKeys.detail(recordType, context), saved)
      void invalidate()
      toast({ title: t('toast.saved'), variant: 'success' })
    } catch (error) {
      const found = describeProblems(error)
      setProblems(found)
      if (found.length === 0) toast({ title: t('toast.error'), variant: 'destructive' })
    }
  }

  const editingRule = editingRules ? (layoutFields(draft).find((field) => field.key === editingRules) ?? null) : null
  const editingField = editingRules ? (schema.fields.find((field) => field.key === editingRules) ?? null) : null

  return (
    <div className="grid min-w-0 gap-4">
      <div className="flex min-w-0 flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <Badge variant={saved ? 'default' : 'secondary'} className="w-fit">
          {saved ? t('status.custom') : t('status.default')}
        </Badge>
        <div className="flex flex-wrap gap-2">
          <Button type="button" variant="outline" onClick={() => setPreviewing(true)}>
            <Eye aria-hidden="true" />
            {t('actions.preview')}
          </Button>
          {dirty && (
            <Button type="button" variant="ghost" onClick={() => setDraft(initial)}>
              {t('actions.discard')}
            </Button>
          )}
          {saved && (
            <Button type="button" variant="outline" onClick={() => setConfirmingReset(true)}>
              <RotateCcw aria-hidden="true" />
              {t('actions.reset')}
            </Button>
          )}
          <Button type="button" onClick={() => void submit()} disabled={save.isPending || (!dirty && Boolean(saved))}>
            {save.isPending && <Loader2 className="animate-spin" aria-hidden="true" />}
            {save.isPending ? t('actions.saving') : t('actions.save')}
          </Button>
        </div>
      </div>
      {problems.length > 0 && (
        <Alert className="grid gap-1 text-destructive">
          <p className="font-medium">{t('errors.invalid')}</p>
          <ul className="list-disc pl-5">
            {problems.map((problem, index) => (
              <li key={index}>{problem}</li>
            ))}
          </ul>
        </Alert>
      )}
      <LayoutCanvas layout={draft} candidates={candidates} context={context} onChange={setDraft} onEditRules={setEditingRules} />
      <FieldRulesDialog
        field={editingField}
        rule={editingRule}
        schema={schema}
        placedKeys={layoutKeys(draft)}
        context={context}
        onClose={() => setEditingRules(null)}
        onApply={(rules) => {
          if (editingRules) setDraft(setFieldRules(draft, editingRules, rules))
          setEditingRules(null)
        }}
      />
      <Dialog open={previewing} onOpenChange={setPreviewing}>
        <DialogContent size="2xl">
          <DialogHeader>
            <DialogTitle>{t('preview.title')}</DialogTitle>
            <DialogDescription>{t('preview.description')}</DialogDescription>
          </DialogHeader>
          {previewing && (
            <RecordForm
              schema={schema}
              formLayout={draft}
              readOnlyKeys={context === 'view' ? [] : undefined}
              mode={context === 'create' ? 'create' : 'edit'}
              record={context === 'create' ? undefined : { id: 'preview' }}
              guardUnsavedChanges={false}
              onCancel={() => setPreviewing(false)}
              onSubmit={() => {
                toast({ title: t('preview.submitted') })
              }}
            />
          )}
        </DialogContent>
      </Dialog>
      <ConfirmDialog
        open={confirmingReset}
        onOpenChange={setConfirmingReset}
        title={t('confirmReset.title')}
        description={t('confirmReset.description', { form: t(`contexts.${context}`) })}
        confirmLabel={t('confirmReset.confirm')}
        variant="destructive"
        onConfirm={async () => {
          await formLayoutsApi.reset(recordType, context)
          await invalidate()
          toast({ title: t('toast.reset'), variant: 'success' })
        }}
      />
      <UnsavedChangesGuard when={dirty && !save.isPending} />
    </div>
  )
}
