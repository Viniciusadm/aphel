import { zodResolver } from '@hookform/resolvers/zod'
import { Plus, Wand2 } from 'lucide-react'
import { useId, useState } from 'react'
import { FormProvider, useFieldArray, useForm, useWatch } from 'react-hook-form'
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
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { defaultPipeline, usePipelines } from '@/modules/pipelines/api'
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
  siteFormTemplate,
  toFormValues,
  toRequestBody,
  webhookFormSchema,
  type WebhookFormValues,
} from './form'
import { Alert } from '@/components/ui/alert'

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
  const pipelines = usePipelines()
  const [newActionType, setNewActionType] = useState<ActionType>('upsert_contact')
  const [serverErrors, setServerErrors] = useState<ServerFieldError[]>([])
  const form = useForm<WebhookFormValues>({
    resolver: zodResolver(webhookFormSchema),
    defaultValues: toFormValues(webhook),
  })
  const { control, register, handleSubmit, formState } = form
  const actions = useFieldArray({ control, name: 'actions' })
  const generateSecret = useWatch({ control, name: 'generate_signing_secret' })
  const signed = webhook ? webhook.secret_configured : generateSecret

  const applyTemplate = () => {
    const pipeline = defaultPipeline(pipelines.data)
    const stage = pipeline?.stages.find((item) => item.is_active)
    actions.replace(siteFormTemplate(pipeline?.id ?? '', stage?.id ?? ''))
  }

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
          <div className="grid gap-2.5">
            <Label htmlFor={`${formId}-name`}>{t('form.name')}</Label>
            <Input
              id={`${formId}-name`}
              maxLength={MAX_NAME_LENGTH}
              placeholder={t('form.namePlaceholder')}
              aria-invalid={formState.errors.name ? true : undefined}
              {...register('name')}
            />
            {formState.errors.name && (
              <p className="text-xs text-destructive">{errorMessage(formState.errors.name.message)}</p>
            )}
          </div>
          <div className="grid gap-2.5">
            <Label htmlFor={`${formId}-description`}>{t('form.descriptionLabel')}</Label>
            <Textarea placeholder={t('placeholders.description')}
              id={`${formId}-description`}
              rows={2}
              maxLength={MAX_DESCRIPTION_LENGTH}
              {...register('description')}
            />
          </div>
          <div className="flex flex-wrap gap-x-6 gap-y-2">
            <label className="flex items-center gap-2 text-sm">
              <input type="checkbox" className="size-4 accent-primary" {...register('is_active')} />
              {t('form.isActive')}
            </label>
            {!webhook && (
              <label className="flex items-center gap-2 text-sm">
                <input type="checkbox" className="size-4 accent-primary" {...register('generate_signing_secret')} />
                {t('form.generateSecret')}
              </label>
            )}
          </div>
          {signed && (
            <div className="grid gap-2.5 sm:max-w-xs">
              <Label htmlFor={`${formId}-tolerance`}>{t('form.tolerance')}</Label>
              <Input placeholder={t('placeholders.tolerance')}
                id={`${formId}-tolerance`}
                type="number"
                min={MIN_TOLERANCE_SECONDS}
                max={MAX_TOLERANCE_SECONDS}
                {...register('signature_tolerance_seconds', { valueAsNumber: true })}
              />
              {formState.errors.signature_tolerance_seconds && (
                <p className="text-xs text-destructive">
                  {errorMessage(formState.errors.signature_tolerance_seconds.message)}
                </p>
              )}
            </div>
          )}

          <section className="grid gap-3" aria-labelledby={`${formId}-actions`}>
            <div className="flex flex-wrap items-center justify-between gap-2">
              <h3 id={`${formId}-actions`} className="text-sm font-semibold">
                {t('form.actions')}
              </h3>
              <Button type="button" size="sm" variant="outline" onClick={applyTemplate}>
                <Wand2 aria-hidden="true" />
                {t('form.useTemplate')}
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">{t('form.actionsHelp')}</p>
            {targets.isPending ? (
              <Skeleton className="h-24 w-full" />
            ) : actions.fields.length === 0 ? (
              <p className="rounded-md border border-dashed p-3 text-sm text-muted-foreground">{t('form.noActions')}</p>
            ) : (
              <ol className="grid gap-3">
                {actions.fields.map((action, index) => (
                  <ActionEditor
                    key={action.id}
                    index={index}
                    total={actions.fields.length}
                    target={targets.data?.find((target) => target.type === action.type)}
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
