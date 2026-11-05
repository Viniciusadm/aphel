import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useId, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { useSession } from '@/app/session/session-context'
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
  const { t } = useTranslation('records-table')
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{mode === 'create' ? t('views.form.createTitle') : t('views.form.editTitle')}</DialogTitle>
          <DialogDescription>
            {mode === 'create' ? t('views.form.createDescription') : t('views.form.editDescription')}
          </DialogDescription>
        </DialogHeader>
        {open && (
          <ViewForm
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
  view?: SavedView
  pending: boolean
  onCancel: () => void
  onSubmit: (values: ViewFormValues, canManage: boolean) => void
}

function ViewForm({ view, pending, onCancel, onSubmit }: ViewFormProps) {
  const { t } = useTranslation('records-table')
  const { t: tCommon } = useTranslation()
  const { hasPermission } = useSession()
  const canManage = hasPermission(SETTINGS_MANAGE)
  const ids = { name: useId(), visibility: useId(), team: useId(), isDefault: useId(), error: useId() }
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
    <form className="grid gap-4" onSubmit={handleSubmit} noValidate>
      <Field label={t('views.form.name')}>
        {(control) => <>
          <Input
          {...control}
          autoFocus
          value={name}
          maxLength={MAX_VIEW_NAME_LENGTH + 10}
          placeholder={t('views.form.namePlaceholder')}
          aria-invalid={submitted && Boolean(nameError)}
          aria-describedby={submitted && nameError ? ids.error : undefined}
          onChange={(event) => setName(event.target.value)}
        />
        {submitted && nameError && (
          <SharedFieldError id={ids.error}>
            {nameError}
          </SharedFieldError>
        )}
        </>}
      </Field>
      <Field label={t('views.visibility.label')}>
        {(control) => <NativeSelect
          {...control}
          value={visibility}
          onChange={(event) => setVisibility(event.target.value as ViewVisibility)}
        >
          {visibilities.map((option) => (
            <option key={option} value={option}>
              {t(`views.visibility.${option}`)}
            </option>
          ))}
        </NativeSelect>}
      </Field>
      {visibility === 'team' && (
        <Field label={t('views.form.team')}>
          {(control) => <>
            <NativeSelect
            {...control}
            value={teamId}
            aria-invalid={submitted && Boolean(teamError)}
            onChange={(event) => setTeamId(event.target.value)}
          >
            <option value="">{t('filters.choose')}</option>
            {teamOptions.map((team) => (
              <option key={team.id} value={team.id}>
                {team.name}
              </option>
            ))}
          </NativeSelect>
          {teams.isSuccess && teamOptions.length === 0 && (
            <p className="text-sm text-muted-foreground">{t('views.form.noTeams')}</p>
          )}
          {submitted && teamError && <SharedFieldError>{teamError}</SharedFieldError>}
          </>}
        </Field>
      )}
      {visibility === 'organization' && canManage && (
        <div className="flex items-start gap-2">
          <Checkbox
            id={ids.isDefault}
            className="mt-0.5"
            checked={isDefault}
            onCheckedChange={(checked) => setIsDefault(checked === true)}
          />
          <Field id={ids.isDefault} label={t('views.form.organizationDefault')}>
            <p className="text-xs text-muted-foreground">{t('views.form.organizationDefaultHint')}</p>
          </Field>
        </div>
      )}
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onCancel}>
          {tCommon('actions.cancel')}
        </Button>
        <Button type="submit" disabled={pending}>
          {pending ? t('views.form.saving') : t('views.form.save')}
        </Button>
      </DialogFooter>
    </form>
  )
}
