import { useMemo, useState } from 'react'
import { Controller, useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
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
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { toast } from '@/components/ui/toast-store'
import { settingsApi, useSettingsMutation, versionOf, type Member, type Team } from '../api'
import { FieldError, FormAlert } from '../components/FieldError'
import { applyApiError } from '../lib/errors'
import { teamSchema, type TeamValues } from '../lib/schemas'
import { zodResolver } from '@hookform/resolvers/zod'
import { parentOptions } from './hierarchy'
import { Checkbox } from '@/components/ui/checkbox'
import { Field } from '@/components/ui/field'
import { SearchInput } from '@/components/ui/search-input'

const NO_PARENT = 'none'

type Assignment = Record<string, { manager: boolean }>

interface TeamEditorDialogProps {
  team: Team | null
  teams: Team[]
  members: Member[]
  onClose: () => void
}

function initialAssignment(team: Team | null): Assignment {
  const assignment: Assignment = {}
  for (const member of team?.members ?? []) assignment[member.user_id] = { manager: member.is_manager }
  return assignment
}

export function TeamEditorDialog({ team, teams, members, onClose }: TeamEditorDialogProps) {
  const { t } = useTranslation('settings')
  const [assignment, setAssignment] = useState<Assignment>(() => initialAssignment(team))
  const [search, setSearch] = useState('')
  const visibleMembers = useMemo(() => {
    const needle = search.trim().toLocaleLowerCase()
    if (!needle) return members
    return members.filter((member) => `${member.display_name} ${member.email}`.toLocaleLowerCase().includes(needle))
  }, [members, search])
  const selectedCount = Object.keys(assignment).length
  const save = useSettingsMutation(async (values: TeamValues) => {
    const request = {
      name: values.name,
      parent_team_id: values.parentTeamId === NO_PARENT ? null : values.parentTeamId,
      members: Object.entries(assignment).map(([user_id, { manager }]) => ({ user_id, is_manager: manager })),
    }
    return team ? settingsApi.updateTeam(team.id, request, versionOf(team)) : settingsApi.createTeam(request)
  })
  const {
    register,
    control,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<TeamValues, unknown, TeamValues>({
    resolver: zodResolver(teamSchema),
    defaultValues: { name: team?.name ?? '', parentTeamId: team?.parent_team_id ?? NO_PARENT },
  })

  const toggleMember = (userId: string, selected: boolean) =>
    setAssignment((current) => {
      const next = { ...current }
      if (selected) next[userId] = { manager: false }
      else delete next[userId]
      return next
    })

  const toggleManager = (userId: string, manager: boolean) =>
    setAssignment((current) => ({ ...current, [userId]: { manager } }))

  const onSubmit = handleSubmit(async (values) => {
    try {
      await save.mutateAsync(values)
      toast({ title: t(team ? 'teams.toast.updated' : 'teams.toast.created'), variant: 'success' })
      onClose()
    } catch (error) {
      applyApiError(error, setError, {
        fields: { name: 'name', parent_team_id: 'parentTeamId' },
        codes: { team_cycle: 'parentTeamId' },
      })
    }
  })

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent size="xl">
        <DialogHeader>
          <DialogTitle>{t(team ? 'teams.editor.editTitle' : 'teams.editor.createTitle')}</DialogTitle>
          <DialogDescription>{t('teams.editor.description')}</DialogDescription>
        </DialogHeader>
        <form className="grid gap-4" onSubmit={onSubmit} noValidate>
          <FormAlert message={errors.root?.server?.message} />
          <div className="grid gap-4 sm:grid-cols-2">
            <Field label={t('teams.editor.name')}>
              {(control) => <>
                <Input placeholder={t('teams.editor.namePlaceholder')}
                {...control}
                aria-invalid={errors.name ? true : undefined}
                aria-describedby={errors.name ? 'team-name-error' : undefined}
                {...register('name')}
              />
              <FieldError id="team-name-error" message={errors.name?.message} />
              </>}
            </Field>
            <Field label={t('teams.editor.parent')}>
              {(fieldControl) => <>
                <Controller
                control={control}
                name="parentTeamId"
                render={({ field }) => (
                  <Select value={field.value} onValueChange={field.onChange}>
                    <SelectTrigger
                      id={fieldControl.id}
                      aria-invalid={errors.parentTeamId ? true : undefined}
                      aria-describedby={errors.parentTeamId ? 'team-parent-error' : undefined}
                    >
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value={NO_PARENT}>{t('teams.editor.noParent')}</SelectItem>
                      {parentOptions(teams, team).map((option) => (
                        <SelectItem key={option.id} value={option.id}>
                          {option.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              />
              <FieldError id="team-parent-error" message={errors.parentTeamId?.message} />
              </>}
            </Field>
          </div>
          <fieldset className="grid gap-2">
            <legend className="mb-2 flex w-full items-center justify-between gap-2 text-sm font-medium">
              <span>{t('teams.editor.members')}</span>
              <span className="font-normal text-muted-foreground">
                {t('teams.editor.selectedCount', { count: selectedCount })}
              </span>
            </legend>
            {members.length > 0 ? (
              <SearchInput
                value={search}
                placeholder={t('teams.editor.search')}
                aria-label={t('teams.editor.search')}
                onValueChange={setSearch}
              />
            ) : null}
            {members.length === 0 ? (
              <p className="text-sm text-muted-foreground">{t('teams.editor.noMembers')}</p>
            ) : visibleMembers.length === 0 ? (
              <p className="text-sm text-muted-foreground">{t('teams.editor.noMatches')}</p>
            ) : (
              <ul className="grid max-h-72 gap-1 overflow-y-auto rounded-lg border p-2">
                {visibleMembers.map((member) => {
                  const current = assignment[member.user_id]
                  return (
                    <li key={member.user_id} className="flex items-center justify-between gap-3 rounded-md px-2 py-1.5">
                      <label className="flex min-w-0 items-center gap-2 text-sm">
                        <Checkbox
                          checked={Boolean(current)}
                          aria-label={`${t('teams.editor.member')}: ${member.display_name}`}
                          onCheckedChange={(checked) => toggleMember(member.user_id, checked === true)}
                        />
                        <span className="truncate">
                          {member.display_name} <span className="text-muted-foreground">{member.email}</span>
                        </span>
                      </label>
                      <label className="flex shrink-0 items-center gap-2 text-sm text-muted-foreground">
                        <Checkbox
                          checked={current?.manager ?? false}
                          disabled={!current}
                          aria-label={`${t('teams.editor.manager')}: ${member.display_name}`}
                          onCheckedChange={(checked) => toggleManager(member.user_id, checked === true)}
                        />
                        {t('teams.editor.manager')}
                      </label>
                    </li>
                  )
                })}
              </ul>
            )}
          </fieldset>
          <DialogFooter>
            <Button variant="outline" onClick={onClose} disabled={isSubmitting}>
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
