import { zodResolver } from '@hookform/resolvers/zod'
import { Plus, Trash2 } from 'lucide-react'
import { useMemo } from 'react'
import { Controller, useFieldArray, useForm, useWatch, type Control, type FieldErrors } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { notifySuccess } from '@/components/feedback'
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
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
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

const BUILTIN_RECORD_TYPES = ['deal', 'contact', 'company'] as const

interface RuleEditorDialogProps {
  rule: DistributionRule | null
  onClose: () => void
}

interface FieldMessageProps {
  id: string
  message?: string
}

function FieldMessage({ id, message }: FieldMessageProps) {
  const text = useLeadDistributionText()
  if (!message) return null
  return (
    <p id={id} className="text-sm text-destructive">
      {text(message)}
    </p>
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
                <div className="grid min-w-48 flex-1 gap-2">
                  <Label htmlFor={`member-${index}-user`}>{t('editor.user')}</Label>
                  <Controller
                    control={control}
                    name={`members.${index}.userId`}
                    render={({ field: input }) => (
                      <Select value={input.value || undefined} onValueChange={input.onChange}>
                        <SelectTrigger
                          id={`member-${index}-user`}
                          aria-invalid={memberErrors?.userId ? true : undefined}
                          aria-describedby={memberErrors?.userId ? `member-${index}-user-error` : undefined}
                        >
                          <SelectValue placeholder={t('editor.selectUser')} />
                        </SelectTrigger>
                        <SelectContent>
                          {users.map((user) => (
                            <SelectItem key={user.id} value={user.id}>
                              {user.display_name}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    )}
                  />
                  <FieldMessage id={`member-${index}-user-error`} message={memberErrors?.userId?.message} />
                </div>
                {weighted && (
                  <div className="grid w-24 gap-2">
                    <Label htmlFor={`member-${index}-weight`}>{t('editor.weight')}</Label>
                    <Controller
                      control={control}
                      name={`members.${index}.weight`}
                      render={({ field: input }) => (
                        <Input placeholder={t('placeholders.weight')}
                          id={`member-${index}-weight`}
                          type="number"
                          min={1}
                          max={MAX_WEIGHT}
                          value={Number.isFinite(input.value) ? input.value : ''}
                          onChange={(event) => input.onChange(event.target.valueAsNumber)}
                          aria-invalid={memberErrors?.weight ? true : undefined}
                        />
                      )}
                    />
                  </div>
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
                  <div className="grid gap-2">
                    <Label htmlFor={`member-${index}-from`}>{t('editor.awayFrom')}</Label>
                    <Controller
                      control={control}
                      name={`members.${index}.awayFrom`}
                      render={({ field: input }) => (
                        <Input id={`member-${index}-from`} type="datetime-local" {...input} />
                      )}
                    />
                  </div>
                  <div className="grid gap-2">
                    <Label htmlFor={`member-${index}-until`}>{t('editor.awayUntil')}</Label>
                    <Controller
                      control={control}
                      name={`members.${index}.awayUntil`}
                      render={({ field: input }) => (
                        <Input
                          id={`member-${index}-until`}
                          type="datetime-local"
                          aria-invalid={memberErrors?.awayUntil ? true : undefined}
                          {...input}
                        />
                      )}
                    />
                    <FieldMessage id={`member-${index}-until-error`} message={memberErrors?.awayUntil?.message} />
                  </div>
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
        <div className="grid gap-2">
          <Label htmlFor="routing-field">{t('editor.routingField')}</Label>
          <Controller
            control={control}
            name="routingField"
            render={({ field: input }) => (
              <Select value={input.value || undefined} onValueChange={input.onChange}>
                <SelectTrigger id="routing-field" aria-invalid={errors.routingField ? true : undefined}>
                  <SelectValue placeholder={t('editor.selectField')} />
                </SelectTrigger>
                <SelectContent>
                  {fieldOptions.map((field) => (
                    <SelectItem key={field.key} value={field.key}>
                      {field.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          />
          <FieldMessage id="routing-field-error" message={errors.routingField?.message} />
        </div>
        <div className="grid gap-2">
          <Label htmlFor="routing-fallback">{t('editor.fallback')}</Label>
          <Controller
            control={control}
            name="fallback"
            render={({ field: input }) => (
              <Select value={input.value} onValueChange={input.onChange}>
                <SelectTrigger id="routing-fallback">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="members">{t('editor.fallbacks.members')}</SelectItem>
                  <SelectItem value="none">{t('editor.fallbacks.none')}</SelectItem>
                </SelectContent>
              </Select>
            )}
          />
        </div>
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
                  <div className="grid gap-2">
                    <Label htmlFor={`route-${index}-values`}>{t('editor.routeValues')}</Label>
                    <Controller
                      control={control}
                      name={`routes.${index}.values`}
                      render={({ field: input }) => (
                        <Input placeholder={t('placeholders.routeValues')}
                          id={`route-${index}-values`}
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
                  </div>
                  <div className="grid gap-2">
                    <Label htmlFor={`route-${index}-team`}>{t('editor.routeTeam')}</Label>
                    <Controller
                      control={control}
                      name={`routes.${index}.teamId`}
                      render={({ field: input }) => (
                        <Select value={input.value} onValueChange={input.onChange}>
                          <SelectTrigger id={`route-${index}-team`} aria-invalid={routeErrors?.teamId ? true : undefined}>
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value={NO_TEAM}>{t('editor.noTeam')}</SelectItem>
                            {(teams.data ?? []).map((team) => (
                              <SelectItem key={team.id} value={team.id}>
                                {team.name}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      )}
                    />
                    <FieldMessage id={`route-${index}-team-error`} message={routeErrors?.teamId?.message} />
                  </div>
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

export function RuleEditorDialog({ rule, onClose }: RuleEditorDialogProps) {
  const { t } = useTranslation('lead_distribution')
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
    formState: { errors, isSubmitting },
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

  const onSubmit = handleSubmit(async (values) => {
    try {
      await save.mutateAsync(values)
      notifySuccess(t(rule ? 'toast.updated' : 'toast.created'))
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
  })

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent size="2xl">
        <DialogHeader>
          <DialogTitle>{t(rule ? 'editor.editTitle' : 'editor.createTitle')}</DialogTitle>
          <DialogDescription>{t('editor.description')}</DialogDescription>
        </DialogHeader>
        <form className="grid gap-5" onSubmit={onSubmit} noValidate>
          <FieldMessage id="rule-form-error" message={errors.root?.server?.message} />
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="grid gap-2">
              <Label htmlFor="rule-name">{t('editor.name')}</Label>
              <Input placeholder={t('placeholders.name')} id="rule-name" aria-invalid={errors.name ? true : undefined} {...register('name')} />
              <FieldMessage id="rule-name-error" message={errors.name?.message} />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="rule-key">{t('editor.key')}</Label>
              <Input placeholder={t('placeholders.key')}
                id="rule-key"
                disabled={!creating}
                aria-invalid={errors.key ? true : undefined}
                aria-describedby="rule-key-hint"
                {...register('key')}
              />
              <p id="rule-key-hint" className="text-xs text-muted-foreground">
                {t('editor.keyHint')}
              </p>
              <FieldMessage id="rule-key-error" message={errors.key?.message} />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="rule-record-type">{t('editor.recordType')}</Label>
              <Controller
                control={control}
                name="recordType"
                render={({ field }) => (
                  <Select value={field.value} onValueChange={field.onChange}>
                    <SelectTrigger id="rule-record-type" aria-invalid={errors.recordType ? true : undefined}>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {recordTypes.map((option) => (
                        <SelectItem key={option.value} value={option.value}>
                          {option.label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              />
              <FieldMessage id="rule-record-type-error" message={errors.recordType?.message} />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="rule-strategy">{t('editor.strategy')}</Label>
              <Controller
                control={control}
                name="strategy"
                render={({ field }) => (
                  <Select value={field.value} onValueChange={field.onChange}>
                    <SelectTrigger id="rule-strategy" aria-describedby="rule-strategy-hint">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {STRATEGIES.map((option) => (
                        <SelectItem key={option} value={option}>
                          {t(`strategies.${option}`)}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              />
              <p id="rule-strategy-hint" className="text-xs text-muted-foreground">
                {t(`strategyHints.${strategy}`)}
              </p>
            </div>
            <div className="grid gap-2 sm:col-span-2">
              <Label htmlFor="rule-description">{t('editor.descriptionField')}</Label>
              <Textarea placeholder={t('placeholders.description')} id="rule-description" rows={2} {...register('description')} />
            </div>
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
