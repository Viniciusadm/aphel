import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { DialogForm } from '@/components/ui/dialog-form'
import {
  Dialog,
  DialogContent,
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
    <DialogForm
      title={t('create.title')}
      description={t('create.description')}
      pending={pending}
      submitLabel={t('create.confirm')}
      cancelLabel={t('actions.cancel')}
      onCancel={() => onOpenChange(false)}
      onSubmit={(event) => {
        event.preventDefault()
        setSubmitted(true)
        if (name.trim()) onSubmit({ name: name.trim(), team_ids: teams })
      }}
    >
      <Field label={t('fields.name')} error={nameError ? t('fields.nameRequired') : undefined}>
        {(control) => 
          <Input placeholder={t('placeholders.pipelineName')}
          {...control}
          value={name}
          maxLength={140}
          aria-invalid={nameError || undefined}
          onChange={(event) => setName(event.target.value)}
        />
}
      </Field>
      <TeamsField selected={teams} onChange={setTeams} />
    </DialogForm>
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
