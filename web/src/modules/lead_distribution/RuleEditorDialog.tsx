import { DialogForm } from '@/components/ui/dialog-form'
import { NativeSelect } from '@/components/ui/native-select'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { zodResolver } from '@hookform/resolvers/zod'
import { Plus, Trash2 } from 'lucide-react'
import { useMemo, useRef, type FormEvent } from 'react'
import { Controller, useFieldArray, useForm, useWatch, type Control, type FieldErrors } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { UnsavedChangesGuard, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Dialog,
  DialogContent,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { useCustomObjects } from '@/features/custom-records/api'
import { useOrganizationUsers } from '@/features/records/api'
import { useFields } from '@/features/settings/fields/api'
import {
  STRATEGIES,
  leadDistributionApi,
  useDistributionMutation,
  useDistributionTeams,
  type DistributionRule,
} from './api'
import {
  MAX_WEIGHT,
  NO_TEAM,
  defaultValues,
  emptyMember,
  emptyRoute,
  ruleFormSchema,
  toRequest,
  type RuleFormValues,
} from './form'
import { useLeadDistributionText, type LeadDistributionKey } from './text'
import { Page } from '@/components/ui/page'
import { Field } from '@/components/ui/field'

const BUILTIN_RECORD_TYPES = ['deals.deal', 'contacts.contact', 'companies.company'] as const

interface RuleEditorDialogProps {
  rule: DistributionRule | null
  onClose: () => void
  presentation?: 'dialog' | 'page'
}

interface FieldMessageProps {
  id: string
  message?: string
}

function FieldMessage({ id, message }: FieldMessageProps) {
  const text = useLeadDistributionText()
  if (!message) return null
  return (
    <SharedFieldError id={id}>
      {text(message)}
    </SharedFieldError>
  )
}

interface UserOption {
  id: string
  display_name: string
}

interface MembersEditorProps {
  control: Control<RuleFormValues>
  errors: FieldErrors<RuleFormValues>
  users: UserOption[]
  weighted: boolean
}

function MembersEditor({ control, errors, users, weighted }: MembersEditorProps) {
  const { t } = useTranslation('lead_distribution')
  const members = useFieldArray({ control, name: 'members' })
  const values = useWatch({ control, name: 'members' })
  return (
    <fieldset className="grid gap-3">
      <legend className="text-sm font-medium">{t('editor.members')}</legend>
      <p className="text-sm text-muted-foreground">{t('editor.membersHint')}</p>
      {members.fields.length === 0 && <p className="text-sm text-muted-foreground">{t('editor.noMembers')}</p>}
      <ul className="grid gap-2">
        {members.fields.map((field, index) => {
          const memberErrors = errors.members?.[index]
          const away = values?.[index]?.away ?? false
          return (
            <li key={field.id} className="grid gap-3 rounded-lg border p-3">
              <div className="flex flex-wrap items-end gap-3">
                <Field label={t('editor.user')} className="min-w-48 flex-1">
                  {(fieldControl) => <>
                    <Controller
                    control={control}
                    name={`members.${index}.userId`}
                    render={({ field: input }) => (
                      <NativeSelect value={input.value || ''} onValueChange={input.onChange} id={fieldControl.id}
                          aria-invalid={memberErrors?.userId ? true : undefined}
                          aria-describedby={memberErrors?.userId ? `member-${index}-user-error` : undefined}><option value="" disabled>{t('editor.selectUser')}</option>
                          {users.map((user) => (
                            <option key={user.id} value={user.id}>
                              {user.display_name}
                            </option>
                          ))}
                        </NativeSelect>
                    )}
                  />
                  <FieldMessage id={`member-${index}-user-error`} message={memberErrors?.userId?.message} />
                  </>}
                </Field>
                {weighted && (
                  <Field label={t('editor.weight')} className="w-24">
                    {(fieldControl) => <Controller
                      control={control}
                      name={`members.${index}.weight`}
                      render={({ field: input }) => (
                        <Input placeholder={t('placeholders.weight')}
                          id={fieldControl.id}
                          type="number"
                          min={1}
                          max={MAX_WEIGHT}
                          value={Number.isFinite(input.value) ? input.value : ''}
                          onChange={(event) => input.onChange(event.target.valueAsNumber)}
                          aria-invalid={memberErrors?.weight ? true : undefined}
                        />
                      )}
                    />}
                  </Field>
                )}
                <Controller
                  control={control}
                  name={`members.${index}.away`}
                  render={({ field: input }) => (
                    <label className="flex h-9 items-center gap-2 text-sm">
                      <Checkbox checked={input.value} onCheckedChange={(checked) => input.onChange(checked === true)} />
                      {t('editor.away')}
                    </label>
                  )}
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  aria-label={t('editor.removeMember')}
                  onClick={() => members.remove(index)}
                >
                  <Trash2 />
                </Button>
              </div>
              {memberErrors?.weight && <FieldMessage id={`member-${index}-weight-error`} message={memberErrors.weight.message} />}
              {away && (
                <div className="grid gap-3 sm:grid-cols-2">
                  <Field label={t('editor.awayFrom')}>
                    {(fieldControl) => <Controller
                      control={control}
                      name={`members.${index}.awayFrom`}
                      render={({ field: input }) => (
                        <Input id={fieldControl.id} type="datetime-local" {...input} />
                      )}
                    />}
                  </Field>
                  <Field label={t('editor.awayUntil')}>
                    {(fieldControl) => <>
                      <Controller
                      control={control}
                      name={`members.${index}.awayUntil`}
                      render={({ field: input }) => (
                        <Input
                          id={fieldControl.id}
                          type="datetime-local"
                          aria-invalid={memberErrors?.awayUntil ? true : undefined}
                          {...input}
                        />
                      )}
                    />
                    <FieldMessage id={`member-${index}-until-error`} message={memberErrors?.awayUntil?.message} />
                    </>}
                  </Field>
                  <p className="text-xs text-muted-foreground sm:col-span-2">{t('editor.awayHint')}</p>
                </div>
              )}
            </li>
          )
        })}
      </ul>
      <FieldMessage id="members-error" message={errors.members?.message ?? errors.members?.root?.message} />
      <Button type="button" variant="outline" className="w-fit" onClick={() => members.append(emptyMember())}>
        <Plus />
        {t('editor.addMember')}
      </Button>
    </fieldset>
  )
}

interface RoutingEditorProps {
  control: Control<RuleFormValues>
  errors: FieldErrors<RuleFormValues>
  users: UserOption[]
  recordType: string
}

function RoutingEditor({ control, errors, users, recordType }: RoutingEditorProps) {
  const { t } = useTranslation('lead_distribution')
  const routes = useFieldArray({ control, name: 'routes' })
  const fields = useFields(recordType)
  const teams = useDistributionTeams()
  const fieldOptions = (fields.data ?? []).filter((field) => field.is_active && field.field_type !== 'user')
  return (
    <div className="grid gap-4">
      <div className="grid gap-3 sm:grid-cols-2">
        <Field label={t('editor.routingField')}>
          {(fieldControl) => <>
            <Controller
            control={control}
            name="routingField"
            render={({ field: input }) => (
              <NativeSelect value={input.value || ''} onValueChange={input.onChange} id={fieldControl.id} aria-invalid={errors.routingField ? true : undefined}><option value="" disabled>{t('editor.selectField')}</option>
                  {fieldOptions.map((field) => (
                    <option key={field.key} value={field.key}>
                      {field.label}
                    </option>
                  ))}
                </NativeSelect>
            )}
          />
          <FieldMessage id="routing-field-error" message={errors.routingField?.message} />
          </>}
        </Field>
        <Field label={t('editor.fallback')}>
          {(fieldControl) => <Controller
            control={control}
            name="fallback"
            render={({ field: input }) => (
              <NativeSelect value={input.value} onValueChange={input.onChange} id={fieldControl.id}>
                  <option value="members">{t('editor.fallbacks.members')}</option>
                  <option value="none">{t('editor.fallbacks.none')}</option>
                </NativeSelect>
            )}
          />}
        </Field>
      </div>
      <fieldset className="grid gap-2">
        <legend className="text-sm font-medium">{t('editor.routes')}</legend>
        <ol className="grid gap-2">
          {routes.fields.map((route, index) => {
            const routeErrors = errors.routes?.[index]
            return (
              <li key={route.id} className="grid gap-3 rounded-lg border p-3">
                <div className="flex items-center justify-between gap-2">
                  <span className="text-sm font-medium">{t('editor.routeLabel', { index: index + 1 })}</span>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    aria-label={t('editor.removeRoute')}
                    onClick={() => routes.remove(index)}
                  >
                    <Trash2 />
                  </Button>
                </div>
                <div className="grid gap-3 sm:grid-cols-2">
                  <Field label={t('editor.routeValues')}>
                    {(fieldControl) => <>
                      <Controller
                      control={control}
                      name={`routes.${index}.values`}
                      render={({ field: input }) => (
                        <Input placeholder={t('placeholders.routeValues')}
                          id={fieldControl.id}
                          aria-invalid={routeErrors?.values ? true : undefined}
                          aria-describedby={`route-${index}-values-hint`}
                          {...input}
                        />
                      )}
                    />
                    <p id={`route-${index}-values-hint`} className="text-xs text-muted-foreground">
                      {t('editor.routeValuesHint')}
                    </p>
                    <FieldMessage id={`route-${index}-values-error`} message={routeErrors?.values?.message} />
                    </>}
                  </Field>
                  <Field label={t('editor.routeTeam')}>
                    {(fieldControl) => <>
                      <Controller
                      control={control}
                      name={`routes.${index}.teamId`}
                      render={({ field: input }) => (
                        <NativeSelect value={input.value} onValueChange={input.onChange} id={fieldControl.id} aria-invalid={routeErrors?.teamId ? true : undefined}>
                            <option value={NO_TEAM}>{t('editor.noTeam')}</option>
                            {(teams.data ?? []).map((team) => (
                              <option key={team.id} value={team.id}>
                                {team.name}
                              </option>
                            ))}
                          </NativeSelect>
                      )}
                    />
                    <FieldMessage id={`route-${index}-team-error`} message={routeErrors?.teamId?.message} />
                    </>}
                  </Field>
                </div>
                <Controller
                  control={control}
                  name={`routes.${index}.userIds`}
                  render={({ field: input }) => (
                    <fieldset className="grid gap-1">
                      <legend className="mb-1 text-sm">{t('editor.routeUsers')}</legend>
                      <ul className="grid max-h-36 gap-1 overflow-y-auto rounded-md border p-2 sm:grid-cols-2">
                        {users.map((user) => (
                          <li key={user.id}>
                            <label className="flex items-center gap-2 text-sm">
                              <Checkbox
                                checked={input.value.includes(user.id)}
                                onCheckedChange={(checked) =>
                                  input.onChange(
                                    checked === true
                                      ? [...input.value, user.id]
                                      : input.value.filter((id) => id !== user.id),
                                  )
                                }
                              />
                              <span className="truncate">{user.display_name}</span>
                            </label>
                          </li>
                        ))}
                      </ul>
                    </fieldset>
                  )}
                />
              </li>
            )
          })}
        </ol>
        <FieldMessage id="routes-error" message={errors.routes?.message ?? errors.routes?.root?.message} />
        <Button type="button" variant="outline" className="w-fit" onClick={() => routes.append(emptyRoute())}>
          <Plus />
          {t('editor.addRoute')}
        </Button>
      </fieldset>
    </div>
  )
}

const serverFieldPaths: Record<string, keyof RuleFormValues> = {
  name: 'name',
  key: 'key',
  record_type: 'recordType',
  strategy: 'strategy',
  members: 'members',
  'routing.field': 'routingField',
  'routing.routes': 'routes',
}

export function RuleEditorDialog({ rule, onClose, presentation = 'dialog' }: RuleEditorDialogProps) {
  const { t } = useTranslation('lead_distribution')
  const finished = useRef(false)
  const users = useOrganizationUsers()
  const objects = useCustomObjects()
  const creating = rule === null
  const save = useDistributionMutation((values: RuleFormValues) => {
    const request = toRequest(values, creating)
    return rule ? leadDistributionApi.update(rule.id, request, rule.updated_at) : leadDistributionApi.create(request)
  })
  const {
    register,
    control,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting, isDirty },
  } = useForm<RuleFormValues>({
    resolver: zodResolver(ruleFormSchema),
    defaultValues: defaultValues(rule, 'deal'),
  })
  const strategy = useWatch({ control, name: 'strategy' })
  const recordType = useWatch({ control, name: 'recordType' })
  const routingEnabled = useWatch({ control, name: 'routingEnabled' })

  const recordTypes = useMemo(() => {
    const builtin = BUILTIN_RECORD_TYPES.map((type) => ({
      value: type as string,
      label: t(`recordTypes.${type}`),
    }))
    const custom = (objects.data ?? []).map((object) => ({ value: object.record_type, label: object.label_plural }))
    return [...builtin, ...custom]
  }, [objects.data, t])

  const onSubmit = (event: FormEvent<HTMLFormElement>) => void handleSubmit(async (values) => {
    try {
      await save.mutateAsync(values)
      notifySuccess(t(rule ? 'toast.updated' : 'toast.created'))
      finished.current = true
      onClose()
    } catch (error) {
      let message: LeadDistributionKey = 'errors.generic'
      if (isApiError(error)) {
        if (error.code === 'lead_distribution_rule_key_taken') {
          setError('key', { type: 'server', message: 'errors.keyTaken' })
          return
        }
        if (error.code === 'version_conflict') message = 'errors.versionConflict'
        const fields = (error.details as { fields?: { field: string; message: string }[] } | null)?.fields ?? []
        for (const violation of fields) {
          const prefix = Object.keys(serverFieldPaths).find(
            (path) => violation.field === path || violation.field.startsWith(`${path}.`),
          )
          if (prefix) {
            setError(serverFieldPaths[prefix] as 'name', { type: 'server', message: 'errors.invalid' })
            message = 'errors.invalid'
          }
        }
      }
      setError('root.server', { type: 'server', message })
    }
  })(event)

  const form = <DialogForm
          presentation={presentation}
          back={presentation === 'page' ? { to: '/settings/lead-distribution' } : undefined}
          onSubmit={onSubmit}
          noValidate
          title={t(rule ? 'editor.editTitle' : 'editor.createTitle')}
          description={t('editor.description')}
          cancelLabel={t('actions.cancel')}
          submitLabel={isSubmitting ? t('actions.saving') : t('actions.save')}
          onCancel={onClose}
          pending={isSubmitting}
          submitDisabled={isSubmitting}
        >
          <FieldMessage id="rule-form-error" message={errors.root?.server?.message} />
          <div className="grid gap-4 sm:grid-cols-2">
            <Field label={t('editor.name')}>
              {(fieldControl) => <>
                <Input placeholder={t('placeholders.name')} id={fieldControl.id} aria-invalid={errors.name ? true : undefined} {...register('name')} />
              <FieldMessage id="rule-name-error" message={errors.name?.message} />
              </>}
            </Field>
            <Field label={t('editor.key')}>
              {(control) => <>
                <Input placeholder={t('placeholders.key')}
                {...control}
                disabled={!creating}
                aria-invalid={errors.key ? true : undefined}
                aria-describedby="rule-key-hint"
                {...register('key')}
              />
              <p id="rule-key-hint" className="text-xs text-muted-foreground">
                {t('editor.keyHint')}
              </p>
              <FieldMessage id="rule-key-error" message={errors.key?.message} />
              </>}
            </Field>
            <Field label={t('editor.recordType')}>
              {(fieldControl) => <>
                <Controller
                control={control}
                name="recordType"
                render={({ field }) => (
                  <NativeSelect value={field.value} onValueChange={field.onChange} id={fieldControl.id} aria-invalid={errors.recordType ? true : undefined}>
                      {recordTypes.map((option) => (
                        <option key={option.value} value={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </NativeSelect>
                )}
              />
              <FieldMessage id="rule-record-type-error" message={errors.recordType?.message} />
              </>}
            </Field>
            <Field label={t('editor.strategy')}>
              {(fieldControl) => <>
                <Controller
                control={control}
                name="strategy"
                render={({ field }) => (
                  <NativeSelect value={field.value} onValueChange={field.onChange} id={fieldControl.id} aria-describedby="rule-strategy-hint">
                      {STRATEGIES.map((option) => (
                        <option key={option} value={option}>
                          {t(`strategies.${option}`)}
                        </option>
                      ))}
                    </NativeSelect>
                )}
              />
              <p id="rule-strategy-hint" className="text-xs text-muted-foreground">
                {t(`strategyHints.${strategy}`)}
              </p>
              </>}
            </Field>
            <Field label={t('editor.descriptionField')} className="sm:col-span-2">
              {(fieldControl) => <Textarea placeholder={t('placeholders.description')} id={fieldControl.id} rows={2} {...register('description')} />}
            </Field>
            <Controller
              control={control}
              name="isActive"
              render={({ field }) => (
                <label className="flex items-center gap-2 text-sm">
                  <Checkbox checked={field.value} onCheckedChange={(checked) => field.onChange(checked === true)} />
                  {t('editor.isActive')}
                </label>
              )}
            />
          </div>
          <MembersEditor
            control={control}
            errors={errors}
            users={users.data ?? []}
            weighted={strategy === 'weighted'}
          />
          <section className="grid gap-3 rounded-lg border p-4">
            <Controller
              control={control}
              name="routingEnabled"
              render={({ field }) => (
                <label className="flex items-center gap-2 text-sm font-medium">
                  <Checkbox checked={field.value} onCheckedChange={(checked) => field.onChange(checked === true)} />
                  {t('editor.routing')}
                </label>
              )}
            />
            <p className="text-sm text-muted-foreground">{t('editor.routingHint')}</p>
            {routingEnabled && (
              <RoutingEditor control={control} errors={errors} users={users.data ?? []} recordType={recordType} />
            )}
          </section>
        </DialogForm>
  if (presentation === 'page') return <Page width="form" className="[&_[data-slot=dialog-footer]]:bottom-[calc(4.5rem+env(safe-area-inset-bottom))] md:[&_[data-slot=dialog-footer]]:bottom-0">
    {form}
    <UnsavedChangesGuard when={() => isDirty && !finished.current} />
  </Page>
  return <Dialog open onOpenChange={(open) => !open && onClose()}><DialogContent size="2xl">{form}</DialogContent></Dialog>
}
