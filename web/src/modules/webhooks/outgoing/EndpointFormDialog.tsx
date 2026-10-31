import { zodResolver } from '@hookform/resolvers/zod'
import type { ParseKeys } from 'i18next'
import { useId } from 'react'
import { Controller, useForm, useWatch, type UseFormSetError } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { z } from 'zod'
import { isApiError } from '@/api'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import {
  ALL_EVENTS,
  MAX_DESCRIPTION_LENGTH,
  MAX_URL_LENGTH,
  groupEvents,
  subscribesToAll,
  useEventCatalog,
  webhooksApi,
  type WebhookEndpoint,
  type WebhookEndpointWithSecret,
} from './api'
import { Alert } from '@/components/ui/alert'
import { Field } from '@/components/ui/field'

type MessageKey = ParseKeys<'webhooks'>

const endpointSchema = z
  .object({
    url: z
      .string()
      .trim()
      .min(1, 'form.errors.urlRequired')
      .max(MAX_URL_LENGTH, 'form.errors.urlTooLong')
      .refine((value) => /^https?:\/\/[^\s]+$/i.test(value), 'form.errors.urlInvalid'),
    description: z.string().trim().max(MAX_DESCRIPTION_LENGTH, 'form.errors.descriptionTooLong'),
    allEvents: z.boolean(),
    events: z.array(z.string()),
    isActive: z.boolean(),
  })
  .refine((values) => values.allEvents || values.events.length > 0, {
    path: ['events'],
    message: 'form.errors.eventsRequired',
  })

type EndpointValues = z.infer<typeof endpointSchema>

const SERVER_ERROR = 'root.server'

function fieldViolations(details: unknown): string[] {
  if (typeof details !== 'object' || details === null) return []
  const fields = (details as { fields?: unknown }).fields
  if (!Array.isArray(fields)) return []
  return fields
    .map((item: unknown) => (typeof item === 'object' && item !== null ? (item as { field?: unknown }).field : null))
    .filter((field): field is string => typeof field === 'string')
}

function applyServerError(error: unknown, setError: UseFormSetError<EndpointValues>) {
  if (!isApiError(error)) {
    setError(SERVER_ERROR, { type: 'server', message: 'form.errors.generic' })
    return
  }
  if (error.code === 'webhook_url_not_allowed') {
    setError('url', { type: 'server', message: 'form.errors.urlNotAllowed' }, { shouldFocus: true })
    return
  }
  if (error.code === 'too_many_webhook_endpoints') {
    setError(SERVER_ERROR, { type: 'server', message: 'form.errors.tooMany' })
    return
  }
  if (error.code === 'version_conflict') {
    setError(SERVER_ERROR, { type: 'server', message: 'form.errors.versionConflict' })
    return
  }
  const fields = fieldViolations(error.details)
  if (fields.includes('url')) {
    setError('url', { type: 'server', message: 'form.errors.urlInvalid' }, { shouldFocus: true })
    return
  }
  if (fields.includes('description')) {
    setError('description', { type: 'server', message: 'form.errors.descriptionTooLong' })
    return
  }
  if (fields.includes('events')) {
    setError('events', { type: 'server', message: 'form.errors.eventsInvalid' })
    return
  }
  setError(SERVER_ERROR, { type: 'server', message: 'form.errors.generic' })
}

function defaultValues(endpoint: WebhookEndpoint | null): EndpointValues {
  return {
    url: endpoint?.url ?? '',
    description: endpoint?.description ?? '',
    allEvents: endpoint ? subscribesToAll(endpoint) : false,
    events: endpoint && !subscribesToAll(endpoint) ? endpoint.events : [],
    isActive: endpoint?.is_active ?? true,
  }
}

interface EndpointFormDialogProps {
  endpoint: WebhookEndpoint | null
  onClose: () => void
  onCreated: (endpoint: WebhookEndpointWithSecret) => void
  onUpdated: () => void
}

export function EndpointFormDialog({ endpoint, onClose, onCreated, onUpdated }: EndpointFormDialogProps) {
  const { t } = useTranslation('webhooks')
  const formId = useId()
  const catalog = useEventCatalog()
  const {
    register,
    control,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<EndpointValues, unknown, EndpointValues>({
    resolver: zodResolver(endpointSchema),
    defaultValues: defaultValues(endpoint),
  })
  const allEvents = useWatch({ control, name: 'allEvents' })
  const message = (value: string | undefined) => (value ? t(value as MessageKey) : null)

  const onSubmit = handleSubmit(async (values) => {
    const events = values.allEvents ? [ALL_EVENTS] : values.events
    const description = values.description || null
    try {
      if (endpoint) {
        await webhooksApi.update(endpoint, {
          url: values.url,
          description,
          events,
          is_active: values.isActive,
        })
        onUpdated()
      } else {
        const created = await webhooksApi.create({
          url: values.url,
          description,
          events,
          is_active: values.isActive,
        })
        onCreated(created)
      }
    } catch (error) {
      applyServerError(error, setError)
    }
  })

  const urlError = message(errors.url?.message)
  const descriptionError = message(errors.description?.message)
  const eventsError = message(errors.events?.message)
  const serverError = message(errors.root?.server?.message)

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent size="xl">
        <DialogHeader>
          <DialogTitle>{t(endpoint ? 'form.editTitle' : 'form.createTitle')}</DialogTitle>
          <DialogDescription>{t('form.description')}</DialogDescription>
        </DialogHeader>
        <form id={formId} className="grid gap-4" noValidate onSubmit={(event) => void onSubmit(event)}>
          {serverError && (
            <Alert className="text-destructive"
            >
              {serverError}
            </Alert>
          )}
          <Field label={t('form.url')}>
            {(control) => <>
              <Input
              {...control}
              type="url"
              inputMode="url"
              autoFocus
              placeholder={t('form.urlPlaceholder')}
              maxLength={MAX_URL_LENGTH}
              aria-invalid={urlError ? true : undefined}
              aria-describedby={`${formId}-url-hint`}
              {...register('url')}
            />
            <p id={`${formId}-url-hint`} className={urlError ? 'text-xs text-destructive' : 'text-xs text-muted-foreground'}>
              {urlError ?? t('form.urlHint')}
            </p>
            </>}
          </Field>
          <Field label={t('form.descriptionLabel')}>
            {(control) => <>
              <Textarea
              {...control}
              rows={2}
              maxLength={MAX_DESCRIPTION_LENGTH}
              placeholder={t('form.descriptionPlaceholder')}
              aria-invalid={descriptionError ? true : undefined}
              aria-describedby={descriptionError ? `${formId}-description-error` : undefined}
              {...register('description')}
            />
            {descriptionError && (
              <p id={`${formId}-description-error`} className="text-sm text-destructive">
                {descriptionError}
              </p>
            )}
            </>}
          </Field>
          <fieldset className="grid gap-2" aria-describedby={eventsError ? `${formId}-events-error` : undefined}>
            <legend className="mb-1 text-sm font-medium">{t('form.events')}</legend>
            <Controller
              control={control}
              name="allEvents"
              render={({ field }) => (
                <label className="flex items-center gap-2 text-sm">
                  <Checkbox checked={field.value} onCheckedChange={(checked) => field.onChange(checked === true)} />
                  {t('form.allEvents')}
                </label>
              )}
            />
            {!allEvents &&
              (catalog.isPending ? (
                <Skeleton className="h-40 w-full" />
              ) : catalog.isError ? (
                <ErrorState onRetry={() => void catalog.refetch()} />
              ) : (
                <Controller
                  control={control}
                  name="events"
                  render={({ field }) => (
                    <div className="grid max-h-72 gap-3 overflow-y-auto rounded-md border p-3">
                      {groupEvents(catalog.data).map((group) => (
                        <div key={group.resource} className="grid gap-1.5">
                          <span className="text-sm font-medium">
                            {t(`resources.${group.resource}` as MessageKey, { defaultValue: group.resource })}
                          </span>
                          <div className="grid gap-1 sm:grid-cols-2">
                            {group.events.map((event) => (
                              <label key={event.type} className="flex items-start gap-2 text-sm" title={event.description}>
                                <Checkbox
                                  className="mt-0.5"
                                  checked={field.value.includes(event.type)}
                                  onCheckedChange={(checked) =>
                                    field.onChange(
                                      checked === true
                                        ? [...field.value.filter((item) => item !== event.type), event.type]
                                        : field.value.filter((item) => item !== event.type),
                                    )
                                  }
                                />
                                <span className="font-mono text-xs break-all">{event.type}</span>
                              </label>
                            ))}
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                />
              ))}
            {eventsError && (
              <p id={`${formId}-events-error`} className="text-sm text-destructive">
                {eventsError}
              </p>
            )}
          </fieldset>
          <Controller
            control={control}
            name="isActive"
            render={({ field }) => (
              <label className="flex items-center gap-2 text-sm">
                <Checkbox checked={field.value} onCheckedChange={(checked) => field.onChange(checked === true)} />
                {t('form.active')}
              </label>
            )}
          />
        </form>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={onClose} disabled={isSubmitting}>
            {t('form.cancel')}
          </Button>
          <Button type="submit" form={formId} disabled={isSubmitting}>
            {isSubmitting ? t('form.saving') : t(endpoint ? 'form.save' : 'form.create')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
