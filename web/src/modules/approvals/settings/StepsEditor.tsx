import { ArrowDown, ArrowUp, Plus, Trash2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { useOrganizationUsers } from '@/features/records/api'
import { useRoles } from '@/features/settings/api'
import { APPROVAL_RULES, APPROVER_KINDS, MAX_POLICY_STEPS, MIN_POLICY_STEPS } from '../constants'
import type { ApprovalRule, ApproverKind } from '../types'
import { newStep, type FormErrors, type StepDraft } from './policy-form'

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
                <div className="grid gap-2.5 sm:col-span-2">
                  <Label htmlFor={`${step.uid}-name`}>{t('form.stepName')}</Label>
                  <Input
                    id={`${step.uid}-name`}
                    value={step.name}
                    maxLength={140}
                    placeholder={t('form.stepNamePlaceholder')}
                    onChange={(event) => update(index, { name: event.target.value })}
                  />
                </div>
                <div className="grid gap-2.5">
                  <Label htmlFor={`${step.uid}-kind`}>{t('form.approverKind')}</Label>
                  <NativeSelect
                    id={`${step.uid}-kind`}
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
                  </NativeSelect>
                </div>
                <div className="grid gap-2.5">
                  <Label htmlFor={`${step.uid}-rule`}>{t('form.rule')}</Label>
                  <NativeSelect
                    id={`${step.uid}-rule`}
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
                  </NativeSelect>
                </div>
                {step.approverKind === 'user' && (
                  <div className="grid gap-2.5 sm:col-span-2">
                    <Label htmlFor={`${step.uid}-user`}>{t('form.approverUser')}</Label>
                    <NativeSelect
                      id={`${step.uid}-user`}
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
                    {userError && <p className="text-xs text-destructive">{userError}</p>}
                  </div>
                )}
                {step.approverKind === 'role' && (
                  <div className="grid gap-2.5 sm:col-span-2">
                    <Label htmlFor={`${step.uid}-role`}>{t('form.approverRole')}</Label>
                    <NativeSelect
                      id={`${step.uid}-role`}
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
                    {roleError && <p className="text-xs text-destructive">{roleError}</p>}
                  </div>
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
