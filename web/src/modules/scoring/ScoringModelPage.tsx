import { Gauge, Plus } from 'lucide-react'
import { useState } from 'react'
import { useParams } from 'react-router'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { notifyError, notifySuccess, PageLoader, UnsavedChangesGuard } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { NumberMaskInput } from '@/components/ui/number-mask-input'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Textarea } from '@/components/ui/textarea'
import { ToggleField } from '@/components/ui/toggle-field'
import { RequirePermission, SettingsCard, useFieldSchema, WIDE_FIELD } from '@/sdk'
import { scoringApi, useInvalidateScoring, useScoringModels } from './api'
import { useScoringRecordTypeLabel } from './labels'
import { SCORING_PATH } from './routes'
import { ScoringCriterionCard } from './ScoringCriterionCard'
import {
  DESCRIPTION_MAX_LENGTH,
  MAX_CRITERIA,
  NAME_MAX_LENGTH,
  draftFromModel,
  newCriterion,
  serverErrors,
  updateRequest,
  validateDraft,
  type CriterionDraft,
  type FormError,
  type FormErrors,
  type ModelDraft,
} from './scoring-form'
import { SCORING_MANAGE, type ScoringCriterionKind, type ScoringModel } from './types'
import { serializeFilter } from '@/sdk'

const LOCAL_ERRORS: FormError[] = ['required', 'invalidKey', 'points', 'cap', 'decay', 'bands', 'filter', 'integer']

export function ScoringModelPage() {
  return (
    <RequirePermission permission={SCORING_MANAGE}>
      <ModelLoader />
    </RequirePermission>
  )
}

function ModelLoader() {
  const { t } = useTranslation('scoring')
  const { modelId } = useParams()
  const models = useScoringModels()
  const back = { to: SCORING_PATH, label: t('edit.back') }

  if (models.isPending) {
    return (
      <Page>
        <PageHeader title={t('title')} back={back} />
        <PageLoader />
      </Page>
    )
  }
  if (models.isError) {
    return (
      <Page>
        <PageHeader title={t('title')} back={back} />
        <ErrorState onRetry={() => void models.refetch()} />
      </Page>
    )
  }
  const model = models.data.find((item) => item.id === modelId)
  if (!model) {
    return (
      <Page>
        <PageHeader title={t('title')} back={back} />
        <EmptyState icon={Gauge} title={t('edit.notFoundTitle')} description={t('edit.notFoundDescription')} />
      </Page>
    )
  }
  return <ModelEditor key={model.updated_at} model={model} />
}

function ModelEditor({ model }: { model: ScoringModel }) {
  const { t } = useTranslation('scoring')
  const invalidate = useInvalidateScoring()
  const recordTypeLabel = useScoringRecordTypeLabel()
  const schema = useFieldSchema(model.record_type)
  const [initial] = useState(() => draftFromModel(model))
  const [initialJson] = useState(() => JSON.stringify(initial))
  const [draft, setDraft] = useState<ModelDraft>(initial)
  const [errors, setErrors] = useState<FormErrors>({})
  const [saving, setSaving] = useState(false)
  const dirty = JSON.stringify(draft) !== initialJson
  const formId = 'scoring-model-form'

  const patch = (next: Partial<ModelDraft>) => setDraft((current) => ({ ...current, ...next }))
  const patchCriterion = (uid: string, next: Partial<CriterionDraft>) =>
    setDraft((current) => ({
      ...current,
      criteria: current.criteria.map((criterion) => (criterion.uid === uid ? { ...criterion, ...next } : criterion)),
    }))
  const moveCriterion = (index: number, offset: number) =>
    setDraft((current) => {
      const target = index + offset
      if (target < 0 || target >= current.criteria.length) return current
      const criteria = [...current.criteria]
      ;[criteria[index], criteria[target]] = [criteria[target], criteria[index]]
      return { ...current, criteria }
    })
  const addCriterion = (kind: ScoringCriterionKind) =>
    setDraft((current) => ({ ...current, criteria: [...current.criteria, newCriterion(kind)] }))

  const error = (field: string): string | undefined => {
    const value = errors[field]
    if (!value) return undefined
    return (LOCAL_ERRORS as string[]).includes(value) ? t(`dialog.errors.${value as FormError}`) : value
  }

  const submit = async () => {
    const local = validateDraft(draft, false)
    if (schema.data) {
      draft.criteria.forEach((criterion, index) => {
        if (criterion.kind === 'profile' && !serializeFilter(criterion.filter, schema.data))
          local[`criteria.${index}.filter`] = 'filter'
      })
    }
    setErrors(local)
    if (Object.keys(local).length > 0) return
    setSaving(true)
    try {
      await scoringApi.update(model.id, updateRequest(draft, schema.data), model.updated_at)
      await invalidate()
      notifySuccess(t('toasts.updated'))
    } catch (failure) {
      if (isApiError(failure) && failure.status === 409) {
        notifyError(failure, { title: t('toasts.conflict') })
      } else {
        const server = isApiError(failure) ? serverErrors(failure.details) : {}
        setErrors(server)
        if (Object.keys(server).length === 0) notifyError(failure)
      }
    } finally {
      setSaving(false)
    }
  }

  const warm = Number(draft.warmFrom)
  const hot = Number(draft.hotFrom)
  const bandsPreview =
    Number.isInteger(warm) && Number.isInteger(hot) && hot > warm
      ? t('dialog.bandsPreview', { warm, warmEnd: hot - 1, hot })
      : null

  return (
    <Page>
      <UnsavedChangesGuard when={dirty} />
      <PageHeader
        title={model.name}
        description={t('edit.description', { type: recordTypeLabel(model.record_type) })}
        back={{ to: SCORING_PATH, label: t('edit.back') }}
        actions={
          <Button type="submit" form={formId} disabled={saving || !dirty}>
            {t('dialog.save')}
          </Button>
        }
      />
      <form
        id={formId}
        className="grid gap-6"
        noValidate
        onSubmit={(event) => {
          event.preventDefault()
          void submit()
        }}
      >
        <SettingsCard title={t('edit.modelTitle')} description={t('edit.modelDescription')} disabled={saving}>
          <Field label={t('dialog.name')} error={error('name')}>
            {(control) => (
              <Input {...control} value={draft.name} maxLength={NAME_MAX_LENGTH} onChange={(event) => patch({ name: event.target.value })} />
            )}
          </Field>
          <Field label={t('dialog.recordType')}>
            {(control) => <Input {...control} value={recordTypeLabel(draft.recordType)} disabled readOnly />}
          </Field>
          <Field label={t('dialog.descriptionLabel')} error={error('description')} className={WIDE_FIELD}>
            {(control) => (
              <Textarea
                {...control}
                rows={2}
                value={draft.description}
                maxLength={DESCRIPTION_MAX_LENGTH}
                placeholder={t('placeholders.description')}
                onChange={(event) => patch({ description: event.target.value })}
              />
            )}
          </Field>
          <ToggleField
            label={t('dialog.primary')}
            description={t('dialog.primaryHint')}
            checked={draft.isPrimary}
            onCheckedChange={(isPrimary) => patch({ isPrimary })}
          />
          <ToggleField
            label={t('dialog.active')}
            description={t('dialog.activeHint')}
            checked={draft.isActive}
            onCheckedChange={(isActive) => patch({ isActive })}
          />
        </SettingsCard>

        <SettingsCard title={t('dialog.bandsTitle')} description={t('dialog.bandsHint')} disabled={saving}>
          <Field label={t('dialog.warmFrom')} error={error('warm_from')}>
            {(control) => (
              <NumberMaskInput
                {...control}
                kind="integer"
                maxLength={5}
                className="w-32"
                placeholder={t('placeholders.warm')}
                value={draft.warmFrom}
                onValueChange={(warmFrom) => patch({ warmFrom })}
              />
            )}
          </Field>
          <Field label={t('dialog.hotFrom')} error={error('hot_from')}>
            {(control) => (
              <NumberMaskInput
                {...control}
                kind="integer"
                maxLength={5}
                className="w-32"
                placeholder={t('placeholders.hot')}
                value={draft.hotFrom}
                onValueChange={(hotFrom) => patch({ hotFrom })}
              />
            )}
          </Field>
          {bandsPreview && <p className={`${WIDE_FIELD} text-sm text-muted-foreground`}>{bandsPreview}</p>}
        </SettingsCard>

        <section className="grid min-w-0 gap-4" aria-labelledby="scoring-criteria-title">
          <div className="grid gap-1">
            <h2 id="scoring-criteria-title" className="text-base font-semibold">
              {t('dialog.criteriaTitle')}
            </h2>
            <p className="text-sm text-muted-foreground">{t('dialog.criteriaHint')}</p>
          </div>
          {draft.criteria.length === 0 && (
            <EmptyState icon={Gauge} title={t('dialog.noCriteriaTitle')} description={t('dialog.noCriteria')} />
          )}
          {draft.criteria.map((criterion, index) => (
            <ScoringCriterionCard
              key={criterion.uid}
              criterion={criterion}
              index={index}
              total={draft.criteria.length}
              schema={schema.data}
              disabled={saving}
              error={error}
              onChange={(next) => patchCriterion(criterion.uid, next)}
              onMove={(offset) => moveCriterion(index, offset)}
              onRemove={() => patch({ criteria: draft.criteria.filter((item) => item.uid !== criterion.uid) })}
            />
          ))}
          <div className="flex flex-wrap gap-2">
            {(['profile', 'behavior'] as ScoringCriterionKind[]).map((kind) => (
              <Button
                key={kind}
                type="button"
                variant="outline"
                disabled={saving || draft.criteria.length >= MAX_CRITERIA}
                onClick={() => addCriterion(kind)}
              >
                <Plus aria-hidden="true" />
                {kind === 'profile' ? t('dialog.addProfile') : t('dialog.addBehavior')}
              </Button>
            ))}
          </div>
        </section>
      </form>
    </Page>
  )
}
