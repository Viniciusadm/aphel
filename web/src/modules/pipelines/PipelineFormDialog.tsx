import { useState } from 'react'
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
import { TeamsField } from './TeamsField'
import { Field } from '@/components/ui/field'

export interface PipelineFormValues {
  name: string
  team_ids: string[]
}

export interface PipelineFormDialogProps {
  open: boolean
  pending?: boolean
  onOpenChange: (open: boolean) => void
  onSubmit: (values: PipelineFormValues) => void
}

function PipelineForm({ pending, onOpenChange, onSubmit }: Omit<PipelineFormDialogProps, 'open'>) {
  const { t } = useTranslation('pipelines')
  const [name, setName] = useState('')
  const [teams, setTeams] = useState<string[]>([])
  const [submitted, setSubmitted] = useState(false)
  const nameError = submitted && !name.trim()

  return (
    <form
      className="grid gap-4"
      onSubmit={(event) => {
        event.preventDefault()
        setSubmitted(true)
        if (name.trim()) onSubmit({ name: name.trim(), team_ids: teams })
      }}
    >
      <DialogHeader>
        <DialogTitle>{t('create.title')}</DialogTitle>
        <DialogDescription>{t('create.description')}</DialogDescription>
      </DialogHeader>
      <Field label={t('fields.name')}>
        {(control) => <>
          <Input placeholder={t('placeholders.pipelineName')}
          {...control}
          value={name}
          maxLength={140}
          aria-invalid={nameError || undefined}
          onChange={(event) => setName(event.target.value)}
        />
        {nameError && <p className="text-sm text-destructive">{t('fields.nameRequired')}</p>}
        </>}
      </Field>
      <TeamsField selected={teams} onChange={setTeams} />
      <DialogFooter>
        <Button type="button" variant="outline" onClick={() => onOpenChange(false)} disabled={pending}>
          {t('actions.cancel')}
        </Button>
        <Button type="submit" disabled={pending}>
          {t('create.confirm')}
        </Button>
      </DialogFooter>
    </form>
  )
}

export function PipelineFormDialog({ open, onOpenChange, ...props }: PipelineFormDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        {open && <PipelineForm {...props} onOpenChange={onOpenChange} />}
      </DialogContent>
    </Dialog>
  )
}
