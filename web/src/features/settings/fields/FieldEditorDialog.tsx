import { useState } from 'react'
import { Controller, useForm, useWatch } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { toast } from '@/components/ui/toast-store'
import { versionOf } from '../api'
import { fieldsApi, sortByPosition, useCreateField, useFieldMutation, type Field } from './api'
import { DraftOptionsEditor } from './components/DraftOptionsEditor'
import { FormulaSettings, formulaSaveMessages } from './formula'
import { FieldOptionsEditor } from './components/FieldOptionsEditor'
import { CheckboxField, ChoiceSelect, FormAlert, FormField, type Choice } from './components/FormControls'
import { describedBy } from './lib/aria'
import { TypeSettings } from './components/TypeSettings'
import { applyFieldsError, fieldFormPath } from './lib/errors'
import {
  FIELD_TYPES,
  compatibleTypes,
  hasOptions,
  isFormulaType,
  supportsSearchable,
  supportsUnique,
} from './lib/field-types'
import {
  createFieldRequest,
  emptyFormValues,
  fieldFormResolver,
  formValuesFromField,
  updateFieldRequest,
  type FieldFormContext,
  type FieldFormValues,
} from './lib/form'
import { keyFromLabel, optionValuesFromLabels } from './lib/identifiers'

interface FieldEditorDialogProps {
  recordType: string
  field: Field | null
  onClose: () => void
}

function activeOptionChoices(field: Field): Choice[] {
  return sortByPosition(field.options)
    .filter((option) => option.is_active)
    .map((option) => ({ value: option.value, label: option.label }))
}

export function FieldEditorDialog({ recordType, field, onClose }: FieldEditorDialogProps) {
  const { t } = useTranslation('fields')
  const system = field?.is_system ?? false
  const existingChoices = field ? activeOptionChoices(field) : []
  const context: FieldFormContext = {
    mode: field ? 'edit' : 'create',
    system,
    existingOptionValues: existingChoices.map((choice) => choice.value),
  }
  const [keyEdited, setKeyEdited] = useState(Boolean(field))
  const [formulaMessages, setFormulaMessages] = useState<string[]>([])
  const create = useCreateField(recordType)
  const update = useFieldMutation(recordType, ({ target, values }: { target: Field; values: FieldFormValues }) =>
    fieldsApi.update(recordType, target.id, updateFieldRequest(values, target), versionOf(target)),
  )
  const {
    register,
    control,
    handleSubmit,
    setError,
    setValue,
    formState: { errors, isSubmitting },
  } = useForm<FieldFormValues, FieldFormContext, FieldFormValues>({
    context,
    resolver: fieldFormResolver,
    defaultValues: field ? formValuesFromField(field) : emptyFormValues(),
  })
  const fieldType = useWatch({ control, name: 'fieldType' })
  const draftOptions = useWatch({ control, name: 'options' })
  const sensitive = useWatch({ control, name: 'isSensitive' })

  const typeChoices = (field ? compatibleTypes(field.field_type) : [...FIELD_TYPES]).map((type) => ({
    value: type,
    label: t(`types.${type}`),
  }))
  const typeLocked = system || typeChoices.length <= 1
  const draftValues = optionValuesFromLabels(draftOptions.map((option) => option.label))
  const optionChoices: Choice[] = field
    ? existingChoices
    : draftOptions.flatMap((option, index) =>
        option.label.trim() ? [{ value: draftValues[index] ?? '', label: option.label.trim() }] : [],
      )

  const onSubmit = handleSubmit(async (values) => {
    setFormulaMessages([])
    try {
      if (field) await update.mutateAsync({ target: field, values })
      else await create.mutateAsync(createFieldRequest(values))
      toast({ title: t(field ? 'toast.updated' : 'toast.created'), variant: 'success' })
      onClose()
    } catch (error) {
      setFormulaMessages(formulaSaveMessages(error))
      applyFieldsError(error, setError, {
        path: (serverPath) => fieldFormPath(serverPath, values.fieldType),
        codes: {
          field_key_taken: 'key',
          incompatible_field_type: 'fieldType',
          duplicate_values: 'isUnique',
          formula_cycle: 'config.expression',
        },
      })
    }
  })

  const labelField = register('label', {
    onChange: (event: { target: { value: string } }) => {
      if (!keyEdited) setValue('key', keyFromLabel(event.target.value))
    },
  })
  const keyField = register('key', { onChange: () => setKeyEdited(true) })
  const keyHint = field ? t('editor.keyLocked') : t('editor.keyHint')
  const typeHint = field ? (typeLocked ? t('editor.typeLocked') : t('editor.typeHint')) : undefined

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-h-[90vh] overflow-y-auto sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t(field ? 'editor.editTitle' : 'editor.createTitle')}</DialogTitle>
          <DialogDescription>{t('editor.description')}</DialogDescription>
        </DialogHeader>
        <form className="grid min-w-0 gap-4" onSubmit={onSubmit} noValidate>
          <FormAlert message={errors.root?.server?.message} />
          {system && (
            <p className="rounded-md border bg-muted/40 px-3 py-2 text-sm text-muted-foreground">
              {t('editor.systemPrivacyNotice')}
            </p>
          )}
          <div className="grid gap-4 sm:grid-cols-2">
            <FormField id="field-label" label={t('editor.label')} error={errors.label?.message}>
              <Input
                id="field-label"
                aria-invalid={errors.label ? true : undefined}
                aria-describedby={describedBy('field-label', errors.label?.message)}
                {...labelField}
              />
            </FormField>
            <FormField id="field-key" label={t('editor.key')} hint={keyHint} error={errors.key?.message}>
              {field ? (
                <Input
                  id="field-key"
                  className="font-mono"
                  value={field.key}
                  disabled
                  readOnly
                  aria-describedby={describedBy('field-key', undefined, keyHint)}
                />
              ) : (
                <Input
                  id="field-key"
                  className="font-mono"
                  aria-invalid={errors.key ? true : undefined}
                  aria-describedby={describedBy('field-key', errors.key?.message, keyHint)}
                  {...keyField}
                />
              )}
            </FormField>
            <Controller
              control={control}
              name="fieldType"
              render={({ field: typeField, fieldState }) => (
                <FormField
                  id="field-type"
                  label={t('editor.type')}
                  hint={typeHint}
                  error={fieldState.error?.message}
                  className="sm:col-span-2"
                >
                  <ChoiceSelect
                    id="field-type"
                    value={typeField.value}
                    choices={typeChoices}
                    disabled={typeLocked}
                    invalid={Boolean(fieldState.error)}
                    describedBy={describedBy('field-type', fieldState.error?.message, typeHint)}
                    onChange={typeField.onChange}
                  />
                </FormField>
              )}
            />
          </div>
          {!system && isFormulaType(fieldType) && (
            <FormulaSettings
              recordType={recordType}
              fieldKey={field?.key}
              control={control}
              register={register}
              setValue={setValue}
              errors={errors}
              serverMessages={formulaMessages}
            />
          )}
          {!system && !isFormulaType(fieldType) && (
            <>
              <fieldset className="grid gap-2">
                <legend className="mb-2 text-sm font-medium">{t('editor.rules')}</legend>
                <CheckboxField
                  id="field-required"
                  label={t('editor.required')}
                  hint={t('editor.requiredHint')}
                  error={errors.isRequired?.message}
                  inputProps={register('isRequired')}
                />
                {supportsUnique(fieldType) && (
                  <CheckboxField
                    id="field-unique"
                    label={t('editor.unique')}
                    hint={t('editor.uniqueHint')}
                    error={errors.isUnique?.message}
                    inputProps={register('isUnique')}
                  />
                )}
                {supportsSearchable(fieldType) && (
                  <CheckboxField
                    id="field-searchable"
                    label={t('editor.searchable')}
                    hint={t('editor.searchableHint')}
                    error={errors.isSearchable?.message}
                    inputProps={register('isSearchable')}
                  />
                )}
              </fieldset>
              {!field && hasOptions(fieldType) && (
                <DraftOptionsEditor control={control} register={register} errors={errors} />
              )}
              {field && hasOptions(fieldType) && <FieldOptionsEditor recordType={recordType} field={field} />}
              <TypeSettings
                fieldType={fieldType}
                control={control}
                register={register}
                errors={errors}
                optionChoices={optionChoices}
              />
            </>
          )}
          <fieldset className="grid gap-2">
            <legend className="mb-2 text-sm font-medium">{t('editor.privacy')}</legend>
            <CheckboxField
              id="field-personal"
              label={t('editor.personal')}
              hint={t('editor.personalHint')}
              error={errors.isPersonal?.message}
              inputProps={register('isPersonal', {
                onChange: (event: { target: { checked: boolean } }) => {
                  if (!event.target.checked && sensitive) setValue('isSensitive', false)
                },
              })}
            />
            <CheckboxField
              id="field-sensitive"
              label={t('editor.sensitive')}
              hint={t('editor.sensitiveHint')}
              error={errors.isSensitive?.message}
              inputProps={register('isSensitive', {
                onChange: (event: { target: { checked: boolean } }) => {
                  if (event.target.checked) setValue('isPersonal', true)
                },
              })}
            />
          </fieldset>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose} disabled={isSubmitting}>
              {t('actions.cancel')}
            </Button>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? t('actions.saving') : t('actions.save')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
