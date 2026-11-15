import { useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { DialogForm } from '@/components/ui/dialog-form'
import { ToggleField } from '@/components/ui/toggle-field'
import { Input } from '@/components/ui/input'
import { NativeSelect, type SavedView, type ViewVisibility } from '@/sdk'
import { MAX_VIEW_NAME_LENGTH, SETTINGS_MANAGE, useViewTeams } from './viewsApi'
import { Field } from '@/components/ui/field'

export interface ViewFormValues {
  name: string
  visibility: ViewVisibility
  teamId: string | null
  isDefault: boolean
}

export interface ViewFormDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  mode: 'create' | 'edit'
  view?: SavedView
  pending?: boolean
  onSubmit: (values: ViewFormValues, canManage: boolean) => void
}

export function ViewFormDialog({ open, onOpenChange, mode, view, pending = false, onSubmit }: ViewFormDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        {open && (
          <ViewForm
            mode={mode}
            view={mode === 'edit' ? view : undefined}
            pending={pending}
            onCancel={() => onOpenChange(false)}
            onSubmit={onSubmit}
          />
        )}
      </DialogContent>
    </Dialog>
  )
}

interface ViewFormProps {
  mode: 'create' | 'edit'
  view?: SavedView
  pending: boolean
  onCancel: () => void
  onSubmit: (values: ViewFormValues, canManage: boolean) => void
}

function ViewForm({ mode, view, pending, onCancel, onSubmit }: ViewFormProps) {
  const { t } = useTranslation('records-table')
  const { t: tCommon } = useTranslation()
  const { hasPermission } = useSession()
  const canManage = hasPermission(SETTINGS_MANAGE)
  const [name, setName] = useState(view?.name ?? '')
  const [visibility, setVisibility] = useState<ViewVisibility>(view?.visibility ?? 'private')
  const [teamId, setTeamId] = useState(view?.team_id ?? '')
  const [isDefault, setIsDefault] = useState(view?.is_default ?? false)
  const [submitted, setSubmitted] = useState(false)
  const teams = useViewTeams(visibility === 'team')

  const visibilities: ViewVisibility[] = ['private']
  if (teams.canShare || view?.visibility === 'team') visibilities.push('team')
  if (canManage || view?.visibility === 'organization') visibilities.push('organization')

  const trimmed = name.trim()
  const nameError = !trimmed
    ? t('views.form.nameRequired')
    : trimmed.length > MAX_VIEW_NAME_LENGTH
      ? t('views.form.nameTooLong', { max: MAX_VIEW_NAME_LENGTH })
      : null
  const teamError = visibility === 'team' && !teamId ? t('views.form.teamRequired') : null
  const knownTeams = teams.data ?? []
  const teamOptions =
    view?.team_id && !knownTeams.some((team) => team.id === view.team_id)
      ? [...knownTeams, { id: view.team_id, name: view.team_id }]
      : knownTeams

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault()
    setSubmitted(true)
    if (nameError || teamError) return
    onSubmit(
      {
        name: trimmed,
        visibility,
        teamId: visibility === 'team' ? teamId : null,
        isDefault: visibility === 'organization' && isDefault,
      },
      canManage,
    )
  }

  return (
    <DialogForm
      noValidate
      onSubmit={handleSubmit}
      title={mode === 'create' ? t('views.form.createTitle') : t('views.form.editTitle')}
      description={mode === 'create' ? t('views.form.createDescription') : t('views.form.editDescription')}
      cancelLabel={tCommon('actions.cancel')}
      submitLabel={pending ? t('views.form.saving') : t('views.form.save')}
      onCancel={onCancel}
      pending={pending}
      submitDisabled={pending}
    >
      <Field label={t('views.form.name')} error={submitted ? nameError : null}>
        {(control) => (
          <Input
            {...control}
            autoFocus
            value={name}
            maxLength={MAX_VIEW_NAME_LENGTH + 10}
            placeholder={t('views.form.namePlaceholder')}
            onChange={(event) => setName(event.target.value)}
          />
        )}
      </Field>
      <Field label={t('views.visibility.label')}>
        {(control) => (
          <NativeSelect
            {...control}
            value={visibility}
            onChange={(event) => setVisibility(event.target.value as ViewVisibility)}
          >
            {visibilities.map((option) => (
              <option key={option} value={option}>
                {t(`views.visibility.${option}`)}
              </option>
            ))}
          </NativeSelect>
        )}
      </Field>
      {visibility === 'team' && (
        <Field
          label={t('views.form.team')}
          description={teams.isSuccess && teamOptions.length === 0 ? t('views.form.noTeams') : undefined}
          error={submitted ? teamError : null}
        >
          {(control) => (
            <NativeSelect {...control} value={teamId} onChange={(event) => setTeamId(event.target.value)}>
              <option value="">{t('filters.choose')}</option>
              {teamOptions.map((team) => (
                <option key={team.id} value={team.id}>
                  {team.name}
                </option>
              ))}
            </NativeSelect>
          )}
        </Field>
      )}
      {visibility === 'organization' && canManage && (
        <ToggleField
          type="switch"
          label={t('views.form.organizationDefault')}
          description={t('views.form.organizationDefaultHint')}
          checked={isDefault}
          onCheckedChange={setIsDefault}
        />
      )}
    </DialogForm>
  )
}
