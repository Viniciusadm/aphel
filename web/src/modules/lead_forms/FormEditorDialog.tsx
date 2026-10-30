import { zodResolver } from '@hookform/resolvers/zod'
import { Plus } from 'lucide-react'
import { useId, useState } from 'react'
import { FormProvider, useFieldArray, useForm, useWatch } from 'react-hook-form'
import { isApiError } from '@/api'
import { notifyError } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { LEAD_FORMS_KEY, FIELD_KINDS, leadFormsApi, useLeadFormOptions, type LeadForm, type LeadFormFieldKind } from './api'
import { FieldEditor } from './FieldEditor'
import {
  MAX_FIELDS,
  defaultFields,
  emptyField,
  leadFormSchema,
  toFormValues,
  toRequestBody,
  type LeadFormValues,
} from './form'
import { FormPreview } from './FormPreview'

interface FormEditorDialogProps {
  open: boolean
  form: LeadForm | null
  onOpenChange: (open: boolean) => void
  onSaved: (form: LeadForm, created: boolean) => void
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
    (item): item is ServerFieldError => typeof item === 'object' && item !== null && 'field' in item && 'message' in item,
  )
}

export function FormEditorDialog({ open, form, onOpenChange, onSaved }: FormEditorDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-5xl">
        {open && <EditorBody form={form} onCancel={() => onOpenChange(false)} onSaved={onSaved} />}
      </DialogContent>
    </Dialog>
  )
}

interface EditorBodyProps {
  form: LeadForm | null
  onCancel: () => void
  onSaved: FormEditorDialogProps['onSaved']
}

function EditorBody({ form, onCancel, onSaved }: EditorBodyProps) {
  const t = useModuleTranslation(LEAD_FORMS_KEY)
  const formId = useId()
  const options = useLeadFormOptions()
  const [newKind, setNewKind] = useState<LeadFormFieldKind>('text')
  const [serverErrors, setServerErrors] = useState<ServerFieldError[]>([])
  const methods = useForm<LeadFormValues>({
    resolver: zodResolver(leadFormSchema),
    defaultValues: toFormValues(
      form,
      defaultFields({
        name: t('defaults.name'),
        email: t('defaults.email'),
        phone: t('defaults.phone'),
        message: t('defaults.message'),
      }),
    ),
  })
  const { control, register, handleSubmit, formState, setValue } = methods
  const fields = useFieldArray({ control, name: 'fields' })
  const values = useWatch({ control }) as LeadFormValues
  const pipelines = options.data?.pipelines ?? []
  const pipeline = pipelines.find((item) => item.id === values.pipeline_id)
  const errorText = (key?: string) => (key ? t(key) : null)
  const rootFieldsError = formState.errors.fields?.message ?? formState.errors.fields?.root?.message

  const submit = handleSubmit(async (formValues) => {
    setServerErrors([])
    try {
      const body = toRequestBody(formValues)
      const saved = form ? await leadFormsApi.update(form.id, body) : await leadFormsApi.create(body)
      onSaved(saved, form === null)
    } catch (error) {
      setServerErrors(serverFieldErrors(error))
      notifyError(error)
    }
  })

  return (
    <>
      <DialogHeader>
        <DialogTitle>{form ? t('editor.editTitle', { name: form.name }) : t('editor.createTitle')}</DialogTitle>
        <DialogDescription>{t('editor.description')}</DialogDescription>
      </DialogHeader>
      {options.isPending ? (
        <Skeleton className="h-64 w-full" />
      ) : (
        <FormProvider {...methods}>
          <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_320px]">
            <form id={formId} className="grid gap-5" noValidate onSubmit={(event) => void submit(event)}>
              <section className="grid gap-3" aria-labelledby={`${formId}-general`}>
                <h3 id={`${formId}-general`} className="text-sm font-semibold">
                  {t('editor.general')}
                </h3>
                <div className="grid gap-1.5">
                  <Label htmlFor={`${formId}-name`}>{t('editor.name')}</Label>
                  <Input id={`${formId}-name`} placeholder={t('editor.namePlaceholder')} {...register('name')} />
                  {formState.errors.name && (
                    <p className="text-xs text-destructive">{errorText(formState.errors.name.message)}</p>
                  )}
                </div>
                <div className="grid gap-1.5">
                  <Label htmlFor={`${formId}-actor`}>{t('editor.actingUser')}</Label>
                  <NativeSelect id={`${formId}-actor`} {...register('acting_user_id')}>
                    <option value="">{t('editor.actingUserCreator')}</option>
                    {(options.data?.users ?? []).map((user) => (
                      <option key={user.id} value={user.id}>
                        {user.display_name} ({user.email})
                      </option>
                    ))}
                  </NativeSelect>
                  <p className="text-xs text-muted-foreground">{t('editor.actingUserHelp')}</p>
                </div>
                <label className="flex items-center gap-2 text-sm">
                  <input type="checkbox" className="size-4 accent-primary" {...register('is_active')} />
                  {t('editor.isActive')}
                </label>
              </section>

              <section className="grid gap-3" aria-labelledby={`${formId}-fields`}>
                <h3 id={`${formId}-fields`} className="text-sm font-semibold">
                  {t('editor.fields')}
                </h3>
                <p className="text-xs text-muted-foreground">{t('editor.fieldsHelp')}</p>
                <ol className="grid gap-3">
                  {fields.fields.map((field, index) => (
                    <FieldEditor
                      key={field.id}
                      index={index}
                      total={fields.fields.length}
                      targets={options.data?.contact_fields ?? []}
                      onMove={(from, to) => fields.move(from, to)}
                      onRemove={() => fields.remove(index)}
                    />
                  ))}
                </ol>
                {rootFieldsError && <p className="text-xs text-destructive">{errorText(rootFieldsError)}</p>}
                {fields.fields.length < MAX_FIELDS && (
                  <div className="flex flex-wrap items-center gap-2">
                    <NativeSelect
                      aria-label={t('editor.newFieldKind')}
                      value={newKind}
                      onChange={(event) => setNewKind(event.target.value as LeadFormFieldKind)}
                    >
                      {FIELD_KINDS.map((kind) => (
                        <option key={kind} value={kind}>
                          {t(`kinds.${kind}`)}
                        </option>
                      ))}
                    </NativeSelect>
                    <Button
                      type="button"
                      size="sm"
                      variant="outline"
                      onClick={() => fields.append(emptyField(newKind, fields.fields.length))}
                    >
                      <Plus aria-hidden="true" />
                      {t('editor.addField')}
                    </Button>
                  </div>
                )}
              </section>

              <section className="grid gap-3" aria-labelledby={`${formId}-contact`}>
                <h3 id={`${formId}-contact`} className="text-sm font-semibold">
                  {t('editor.contact')}
                </h3>
                <label className="flex items-center gap-2 text-sm">
                  <input type="checkbox" className="size-4 accent-primary" {...register('dedupe_by_email')} />
                  {t('editor.dedupe')}
                </label>
                <div className="grid gap-3 sm:grid-cols-2">
                  <div className="grid gap-1.5">
                    <Label htmlFor={`${formId}-source`}>{t('editor.source')}</Label>
                    <Input id={`${formId}-source`} placeholder={t('editor.sourcePlaceholder')} {...register('source')} />
                  </div>
                  <div className="grid gap-1.5">
                    <Label htmlFor={`${formId}-owner`}>{t('editor.owner')}</Label>
                    <NativeSelect id={`${formId}-owner`} {...register('owner_id')}>
                      <option value="">{t('editor.ownerActingUser')}</option>
                      {(options.data?.users ?? []).map((user) => (
                        <option key={user.id} value={user.id}>
                          {user.display_name} ({user.email})
                        </option>
                      ))}
                    </NativeSelect>
                  </div>
                  <div className="grid gap-1.5">
                    <Label htmlFor={`${formId}-distribution`}>{t('editor.distributionRule')}</Label>
                    <NativeSelect id={`${formId}-distribution`} {...register('distribution_rule_id')}>
                      <option value="">{t('editor.noDistributionRule')}</option>
                      {(options.data?.distribution_rules ?? []).map((rule) => (
                        <option key={rule.id} value={rule.id}>
                          {rule.name}
                        </option>
                      ))}
                    </NativeSelect>
                  </div>
                  <p className="text-xs text-muted-foreground sm:col-span-2">{t('editor.ownerHelp')}</p>
                  <div className="grid gap-1.5">
                    <Label htmlFor={`${formId}-tag`}>{t('editor.tag')}</Label>
                    <NativeSelect id={`${formId}-tag`} {...register('tag_id')}>
                      <option value="">{t('editor.noTag')}</option>
                      {(options.data?.tags ?? []).map((tag) => (
                        <option key={tag.id} value={tag.id}>
                          {tag.name}
                        </option>
                      ))}
                    </NativeSelect>
                  </div>
                </div>
              </section>

              <section className="grid gap-3" aria-labelledby={`${formId}-deal`}>
                <h3 id={`${formId}-deal`} className="text-sm font-semibold">
                  {t('editor.deal')}
                </h3>
                <label className="flex items-center gap-2 text-sm">
                  <input type="checkbox" className="size-4 accent-primary" {...register('create_deal')} />
                  {t('editor.createDeal')}
                </label>
                {values.create_deal && (
                  <div className="grid gap-3 sm:grid-cols-2">
                    <div className="grid gap-1.5">
                      <Label htmlFor={`${formId}-pipeline`}>{t('editor.pipeline')}</Label>
                      <NativeSelect
                        id={`${formId}-pipeline`}
                        {...register('pipeline_id', {
                          onChange: () => setValue('stage_id', ''),
                        })}
                      >
                        <option value="">{t('editor.choose')}</option>
                        {pipelines.map((item) => (
                          <option key={item.id} value={item.id}>
                            {item.name}
                          </option>
                        ))}
                      </NativeSelect>
                      {formState.errors.pipeline_id && (
                        <p className="text-xs text-destructive">{errorText(formState.errors.pipeline_id.message)}</p>
                      )}
                    </div>
                    <div className="grid gap-1.5">
                      <Label htmlFor={`${formId}-stage`}>{t('editor.stage')}</Label>
                      <NativeSelect id={`${formId}-stage`} disabled={!pipeline} {...register('stage_id')}>
                        <option value="">{t('editor.choose')}</option>
                        {(pipeline?.stages ?? []).map((stage) => (
                          <option key={stage.id} value={stage.id}>
                            {stage.name}
                          </option>
                        ))}
                      </NativeSelect>
                      {formState.errors.stage_id && (
                        <p className="text-xs text-destructive">{errorText(formState.errors.stage_id.message)}</p>
                      )}
                    </div>
                    <div className="grid gap-1.5">
                      <Label htmlFor={`${formId}-title`}>{t('editor.titleTemplate')}</Label>
                      <Input id={`${formId}-title`} {...register('title_template')} />
                      <p className="text-xs text-muted-foreground">{t('editor.titleTemplateHelp', { field: '{{name}}', form: '{{form}}' })}</p>
                      {formState.errors.title_template && (
                        <p className="text-xs text-destructive">{errorText(formState.errors.title_template.message)}</p>
                      )}
                    </div>
                    <div className="grid gap-1.5">
                      <Label htmlFor={`${formId}-amount`}>{t('editor.amount')}</Label>
                      <Input id={`${formId}-amount`} inputMode="decimal" {...register('amount')} />
                      {formState.errors.amount && (
                        <p className="text-xs text-destructive">{errorText(formState.errors.amount.message)}</p>
                      )}
                    </div>
                  </div>
                )}
              </section>

              <section className="grid gap-3" aria-labelledby={`${formId}-after`}>
                <h3 id={`${formId}-after`} className="text-sm font-semibold">
                  {t('editor.afterSubmit')}
                </h3>
                <div className="grid gap-1.5">
                  <Label htmlFor={`${formId}-submit-label`}>{t('editor.submitLabel')}</Label>
                  <Input
                    id={`${formId}-submit-label`}
                    placeholder={t('preview.submit')}
                    {...register('submit_label')}
                  />
                </div>
                <div className="grid gap-1.5">
                  <Label htmlFor={`${formId}-success`}>{t('editor.successMessage')}</Label>
                  <Textarea
                    id={`${formId}-success`}
                    rows={2}
                    placeholder={t('preview.defaultSuccess')}
                    {...register('success_message')}
                  />
                </div>
                <div className="grid gap-1.5">
                  <Label htmlFor={`${formId}-redirect`}>{t('editor.redirectUrl')}</Label>
                  <Input
                    id={`${formId}-redirect`}
                    type="url"
                    placeholder="https://www.exemplo.com.br/obrigado"
                    {...register('redirect_url')}
                  />
                  {formState.errors.redirect_url && (
                    <p className="text-xs text-destructive">{errorText(formState.errors.redirect_url.message)}</p>
                  )}
                </div>
              </section>

              <section className="grid gap-3" aria-labelledby={`${formId}-security`}>
                <h3 id={`${formId}-security`} className="text-sm font-semibold">
                  {t('editor.security')}
                </h3>
                <div className="grid gap-1.5">
                  <Label htmlFor={`${formId}-origins`}>{t('editor.allowedOrigins')}</Label>
                  <Textarea
                    id={`${formId}-origins`}
                    rows={3}
                    className="font-mono text-xs"
                    placeholder="https://www.exemplo.com.br"
                    {...register('allowed_origins')}
                  />
                  <p className="text-xs text-muted-foreground">{t('editor.allowedOriginsHelp')}</p>
                  {formState.errors.allowed_origins && (
                    <p className="text-xs text-destructive">{errorText(formState.errors.allowed_origins.message)}</p>
                  )}
                </div>
              </section>

              {serverErrors.length > 0 && (
                <ul
                  role="alert"
                  className="grid gap-1 rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-xs text-destructive"
                >
                  {serverErrors.map((error) => (
                    <li key={`${error.field}-${error.message}`}>
                      <code>{error.field}</code>: {error.message}
                    </li>
                  ))}
                </ul>
              )}
            </form>
            <aside className="grid content-start gap-2 lg:sticky lg:top-0">
              <h3 className="text-sm font-semibold">{t('preview.title')}</h3>
              <FormPreview values={values} />
            </aside>
          </div>
        </FormProvider>
      )}
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onCancel}>
          {t('editor.cancel')}
        </Button>
        <Button type="submit" form={formId} disabled={formState.isSubmitting || options.isPending}>
          {form ? t('editor.save') : t('editor.create')}
        </Button>
      </DialogFooter>
    </>
  )
}
