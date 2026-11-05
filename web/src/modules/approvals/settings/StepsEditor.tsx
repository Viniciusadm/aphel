import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { ArrowDown, ArrowUp, Plus, Trash2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { NativeSelect, useOrganizationUsers, useRoles } from '@/sdk'
import {APPROVAL_RULES, APPROVER_KINDS, MAX_POLICY_STEPS, MIN_POLICY_STEPS} from '@/sdk'
import type {ApprovalRule, ApproverKind} from '@/sdk'
import { newStep, type FormErrors, type StepDraft } from './policy-form'
import { Field } from '@/components/ui/field'

export interface StepsEditorProps {
  steps: StepDraft[]
  errors: FormErrors
  errorText: (value: string | undefined) => string | null
  onChange: (steps: StepDraft[]) => void
}

export function StepsEditor({ steps, errors, errorText, onChange }: StepsEditorProps) {
  const { t } = useTranslation('approvals')
  const users = useOrganizationUsers()
  const roles = useRoles()

  const update = (index: number, patch: Partial<StepDraft>) =>
    onChange(steps.map((step, position) => (position === index ? { ...step, ...patch } : step)))

  const move = (index: number, offset: number) => {
    const target = index + offset
    if (target < 0 || target >= steps.length) return
    const next = [...steps]
    const [item] = next.splice(index, 1)
    next.splice(target, 0, item)
    onChange(next)
  }

  return (
    <fieldset className="grid gap-3">
      <legend className="mb-2 text-sm font-medium">{t('form.steps')}</legend>
      <ol className="grid gap-3">
        {steps.map((step, index) => {
          const base = `steps.${index}`
          const userError = errorText(errors[`${base}.approver_user_id`])
          const roleError = errorText(errors[`${base}.approver_role_id`])
          const stepError = errorText(errors[base] ?? errors[`${base}.approver_kind`] ?? errors[`${base}.rule`])
          return (
            <li key={step.uid} className="grid gap-3 rounded-md border p-3">
              <div className="flex items-center gap-2">
                <span className="text-sm font-medium">{t('request.stepNumber', { number: index + 1 })}</span>
                <div className="ml-auto flex gap-1">
                  <Button
                    type="button"
                    size="icon"
                    variant="ghost"
                    className="size-8"
                    aria-label={t('form.moveStepUp')}
                    disabled={index === 0}
                    onClick={() => move(index, -1)}
                  >
                    <ArrowUp aria-hidden="true" />
                  </Button>
                  <Button
                    type="button"
                    size="icon"
                    variant="ghost"
                    className="size-8"
                    aria-label={t('form.moveStepDown')}
                    disabled={index === steps.length - 1}
                    onClick={() => move(index, 1)}
                  >
                    <ArrowDown aria-hidden="true" />
                  </Button>
                  <Button
                    type="button"
                    size="icon"
                    variant="ghost"
                    className="size-8"
                    aria-label={t('form.removeStep')}
                    disabled={steps.length <= MIN_POLICY_STEPS}
                    onClick={() => onChange(steps.filter((_, position) => position !== index))}
                  >
                    <Trash2 aria-hidden="true" />
                  </Button>
                </div>
              </div>
              <div className="grid gap-3 sm:grid-cols-2">
                <Field label={t('form.stepName')} className="sm:col-span-2">
                  {(control) => <Input
                    {...control}
                    value={step.name}
                    maxLength={140}
                    placeholder={t('form.stepNamePlaceholder')}
                    onChange={(event) => update(index, { name: event.target.value })}
                  />}
                </Field>
                <Field label={t('form.approverKind')}>
                  {(control) => <NativeSelect
                    {...control}
                    value={step.approverKind}
                    onChange={(event) =>
                      update(index, {
                        approverKind: event.target.value as ApproverKind,
                      })
                    }
                  >
                    {APPROVER_KINDS.map((kind) => (
                      <option key={kind} value={kind}>
                        {t(`approverKinds.${kind}`)}
                      </option>
                    ))}
                  </NativeSelect>}
                </Field>
                <Field label={t('form.rule')}>
                  {(control) => <NativeSelect
                    {...control}
                    value={step.rule}
                    onChange={(event) =>
                      update(index, {
                        rule: event.target.value as ApprovalRule,
                      })
                    }
                  >
                    {APPROVAL_RULES.map((rule) => (
                      <option key={rule} value={rule}>
                        {t(`rules.${rule}`)}
                      </option>
                    ))}
                  </NativeSelect>}
                </Field>
                {step.approverKind === 'user' && (
                  <Field label={t('form.approverUser')} className="sm:col-span-2">
                    {(control) => <>
                      <NativeSelect
                      {...control}
                      value={step.approverUserId}
                      aria-invalid={userError ? true : undefined}
                      onChange={(event) => update(index, { approverUserId: event.target.value })}
                    >
                      <option value="">{t('form.choose')}</option>
                      {(users.data ?? []).map((user) => (
                        <option key={user.id} value={user.id}>
                          {user.display_name}
                        </option>
                      ))}
                    </NativeSelect>
                    {userError && <SharedFieldError>{userError}</SharedFieldError>}
                    </>}
                  </Field>
                )}
                {step.approverKind === 'role' && (
                  <Field label={t('form.approverRole')} className="sm:col-span-2">
                    {(control) => <>
                      <NativeSelect
                      {...control}
                      value={step.approverRoleId}
                      aria-invalid={roleError ? true : undefined}
                      onChange={(event) => update(index, { approverRoleId: event.target.value })}
                    >
                      <option value="">{t('form.choose')}</option>
                      {(roles.data ?? []).map((role) => (
                        <option key={role.id} value={role.id}>
                          {role.name}
                        </option>
                      ))}
                    </NativeSelect>
                    {roleError && <SharedFieldError>{roleError}</SharedFieldError>}
                    </>}
                  </Field>
                )}
                {step.approverKind === 'owner_team_manager' && (
                  <p className="text-xs text-muted-foreground sm:col-span-2">{t('form.ownerTeamManagerHint')}</p>
                )}
                {stepError && <p className="text-xs text-destructive sm:col-span-2">{stepError}</p>}
              </div>
            </li>
          )
        })}
      </ol>
      <Button
        type="button"
        variant="outline"
        size="sm"
        className="w-fit"
        disabled={steps.length >= MAX_POLICY_STEPS}
        onClick={() => onChange([...steps, newStep()])}
      >
        <Plus aria-hidden="true" />
        {t('form.addStep')}
      </Button>
    </fieldset>
  )
}
