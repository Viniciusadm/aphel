import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { zodResolver } from '@hookform/resolvers/zod'
import { Plus } from 'lucide-react'
import { useId, useState } from 'react'
import { Controller, FormProvider, useFieldArray, useForm, useWatch } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { notifyError } from '@/components/feedback'
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
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { ActionEditor } from './ActionEditor'
import {
  ACTION_TYPES,
  incomingWebhooksApi,
  useActionTargets,
  type ActionType,
  type IncomingWebhook,
  type IssuedIncomingWebhook,
} from './api'
import {
  MAX_ACTIONS,
  MAX_DESCRIPTION_LENGTH,
  MAX_NAME_LENGTH,
  MAX_TOLERANCE_SECONDS,
  MIN_TOLERANCE_SECONDS,
  emptyAction,
  toFormValues,
  toRequestBody,
  webhookFormSchema,
  type WebhookFormValues,
} from './form'
import { Alert } from '@/components/ui/alert'
import { EmptyState } from '@/components/ui/empty-state'
import { Checkbox } from '@/components/ui/checkbox'
import { Field } from '@/components/ui/field'

interface WebhookEditorDialogProps {
  open: boolean
  webhook: IncomingWebhook | null
  onOpenChange: (open: boolean) => void
  onSaved: (result: { webhook: IncomingWebhook; issued: IssuedIncomingWebhook | null }) => void
}

interface ServerFieldError {
  field: string
  message: string
}

function serverFieldErrors(error: unknown): ServerFieldError[] {
  if (!isApiError(error) || typeof error.details !== 'object' || error.details === null) return []
  const fields = (error.details as { fields?: unknown }).fields
  if (!Array.isArray(fields)) return []
  return fields.filter(
    (item): item is ServerFieldError =>
      typeof item === 'object' && item !== null && 'field' in item && 'message' in item,
  )
}

export function WebhookEditorDialog({ open, webhook, onOpenChange, onSaved }: WebhookEditorDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="2xl">
        <EditorBody webhook={webhook} onCancel={() => onOpenChange(false)} onSaved={onSaved} />
      </DialogContent>
    </Dialog>
  )
}

interface EditorBodyProps {
  webhook: IncomingWebhook | null
  onCancel: () => void
  onSaved: WebhookEditorDialogProps['onSaved']
}

function EditorBody({ webhook, onCancel, onSaved }: EditorBodyProps) {
  const { t } = useTranslation('webhooks', { keyPrefix: 'incoming' })
  const formId = useId()
  const targets = useActionTargets()
  const [newActionType, setNewActionType] = useState<ActionType>('create_record')
  const [serverErrors, setServerErrors] = useState<ServerFieldError[]>([])
  const form = useForm<WebhookFormValues>({
    resolver: zodResolver(webhookFormSchema),
    defaultValues: toFormValues(webhook),
  })
  const { control, register, handleSubmit, formState } = form
  const actions = useFieldArray({ control, name: 'actions' })
  const generateSecret = useWatch({ control, name: 'generate_signing_secret' })
  const signed = webhook ? webhook.secret_configured : generateSecret

  const submit = handleSubmit(async (values) => {
    setServerErrors([])
    try {
      const body = toRequestBody(values)
      if (webhook) {
        const { generate_signing_secret: _ignored, ...update } = body
        const saved = await incomingWebhooksApi.update(webhook.id, update, webhook.updated_at)
        onSaved({ webhook: saved, issued: null })
      } else {
        const issued = await incomingWebhooksApi.create(body)
        onSaved({ webhook: issued, issued })
      }
    } catch (error) {
      setServerErrors(serverFieldErrors(error))
      notifyError(error)
    }
  })

  const errorMessage = (message?: string) => (message ? t(message, { defaultValue: message }) : null)

  return (
    <>
      <DialogHeader>
        <DialogTitle>{webhook ? t('form.editTitle', { name: webhook.name }) : t('form.createTitle')}</DialogTitle>
        <DialogDescription>{t('form.description')}</DialogDescription>
      </DialogHeader>
      <FormProvider {...form}>
        <form id={formId} className="grid gap-4" noValidate onSubmit={(event) => void submit(event)}>
          <Field label={t('form.name')}>
            {(control) => <>
              <Input
              {...control}
              maxLength={MAX_NAME_LENGTH}
              placeholder={t('form.namePlaceholder')}
              aria-invalid={formState.errors.name ? true : undefined}
              {...register('name')}
            />
            {formState.errors.name && (
              <SharedFieldError>{errorMessage(formState.errors.name.message)}</SharedFieldError>
            )}
            </>}
          </Field>
          <Field label={t('form.descriptionLabel')}>
            {(control) => <Textarea placeholder={t('placeholders.description')}
              {...control}
              rows={2}
              maxLength={MAX_DESCRIPTION_LENGTH}
              {...register('description')}
            />}
          </Field>
          <div className="flex flex-wrap gap-x-6 gap-y-2">
            <label className="flex items-center gap-2 text-sm">
              <Controller
          control={control}
          name="is_active"
          render={({ field }) => (
            <Checkbox checked={field.value === true} onCheckedChange={(checked) => field.onChange(checked === true)} />
          )}
        />
              {t('form.isActive')}
            </label>
            {!webhook && (
              <label className="flex items-center gap-2 text-sm">
                <Controller
          control={control}
          name="generate_signing_secret"
          render={({ field }) => (
            <Checkbox checked={field.value === true} onCheckedChange={(checked) => field.onChange(checked === true)} />
          )}
        />
                {t('form.generateSecret')}
              </label>
            )}
          </div>
          {signed && (
            <Field label={t('form.tolerance')} className="sm:max-w-xs">
              {(control) => <>
                <Input placeholder={t('placeholders.tolerance')}
                {...control}
                type="number"
                min={MIN_TOLERANCE_SECONDS}
                max={MAX_TOLERANCE_SECONDS}
                {...register('signature_tolerance_seconds', { valueAsNumber: true })}
              />
              {formState.errors.signature_tolerance_seconds && (
                <SharedFieldError>
                  {errorMessage(formState.errors.signature_tolerance_seconds.message)}
                </SharedFieldError>
              )}
              </>}
            </Field>
          )}

          <section className="grid gap-3" aria-labelledby={`${formId}-actions`}>
            <div className="flex flex-wrap items-center justify-between gap-2">
              <h3 id={`${formId}-actions`} className="text-sm font-semibold">
                {t('form.actions')}
              </h3>

            </div>
            <p className="text-xs text-muted-foreground">{t('form.actionsHelp')}</p>
            {targets.isPending ? (
              <Skeleton className="h-24 w-full" />
            ) : actions.fields.length === 0 ? (
              <EmptyState title={t('form.noActions')} />
            ) : (
              <ol className="grid gap-3">
                {actions.fields.map((action, index) => (
                  <ActionEditor
                    key={action.id}
                    index={index}
                    total={actions.fields.length}
                    onMove={(from, to) => actions.move(from, to)}
                    onRemove={() => actions.remove(index)}
                  />
                ))}
              </ol>
            )}
            {actions.fields.length < MAX_ACTIONS && (
              <div className="flex flex-wrap items-center gap-2">
                <NativeSelect
                  aria-label={t('form.newActionType')}
                  value={newActionType}
                  onChange={(event) => setNewActionType(event.target.value as ActionType)}
                >
                  {ACTION_TYPES.map((type) => (
                    <option key={type} value={type}>
                      {t(`actionTypes.${type}`)}
                    </option>
                  ))}
                </NativeSelect>
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={() => actions.append(emptyAction(newActionType))}
                >
                  <Plus aria-hidden="true" />
                  {t('form.addAction')}
                </Button>
              </div>
            )}
          </section>

          {serverErrors.length > 0 && (
            <Alert className="grid gap-1 text-xs text-destructive"
            >
              {serverErrors.map((error) => (
                <li key={`${error.field}-${error.message}`}>
                  <code>{error.field}</code>: {error.message}
                </li>
              ))}
            </Alert>
          )}
        </form>
      </FormProvider>
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onCancel}>
          {t('form.cancel')}
        </Button>
        <Button type="submit" form={formId} disabled={formState.isSubmitting}>
          {webhook ? t('form.save') : t('form.create')}
        </Button>
      </DialogFooter>
    </>
  )
}
