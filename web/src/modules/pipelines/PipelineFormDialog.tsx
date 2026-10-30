import { useId, useState } from 'react'
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
import { Label } from '@/components/ui/label'
import { TeamsField } from './TeamsField'

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
  const nameId = useId()
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
      <div className="grid gap-2">
        <Label htmlFor={nameId}>{t('fields.name')}</Label>
        <Input
          id={nameId}
          value={name}
          maxLength={140}
          aria-invalid={nameError || undefined}
          onChange={(event) => setName(event.target.value)}
        />
        {nameError && <p className="text-sm text-destructive">{t('fields.nameRequired')}</p>}
      </div>
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
