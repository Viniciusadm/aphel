import { useMutation } from '@tanstack/react-query'
import { ArrowLeft, FilePlus2, Sparkles } from 'lucide-react'
import { useId, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import type { ReferenceOption } from '@/kernel/modules/define'
import { ReferenceOptions } from '@/kernel/modules/ReferenceOptions'
import { automationPath, automationsApi, useAutomationCache } from './api'
import { IssueList } from './components/IssueList'
import { reportFromError } from './lib/errors'
import { AUTOMATION_EXAMPLES, type AutomationExample } from './lib/examples'
import { supportsRecordType, useAutomationLabels } from './lib/labels'
import type { AutomationCatalog, CreateAutomationPayload } from './types'

export interface CreateAutomationDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  catalog: AutomationCatalog
  recordTypes: string[]
}

type Choice = { kind: 'blank' } | { kind: 'example'; example: AutomationExample }

const STAGE_CONTEXT = { includeInactive: true }

export function CreateAutomationDialog({ open, onOpenChange, catalog, recordTypes }: CreateAutomationDialogProps) {
  const { t } = useTranslation('automations')
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent size="xl">
        <DialogHeader>
          <DialogTitle>{t('create.title')}</DialogTitle>
          <DialogDescription>{t('create.description')}</DialogDescription>
        </DialogHeader>
        {open && <CreateAutomationBody catalog={catalog} recordTypes={recordTypes} onDone={() => onOpenChange(false)} />}
      </DialogContent>
    </Dialog>
  )
}

function CreateAutomationBody({
  catalog,
  recordTypes,
  onDone,
}: {
  catalog: AutomationCatalog
  recordTypes: string[]
  onDone: () => void
}) {
  const { t } = useTranslation('automations')
  const labels = useAutomationLabels(catalog)
  const [choice, setChoice] = useState<Choice | null>(null)
  const available = (example: AutomationExample) =>
    recordTypes.includes(example.recordType) && catalog.triggers.some((trigger) => trigger.key === example.trigger)

  if (!choice) {
    return (
      <div className="grid gap-3">
        <ul className="grid gap-2 sm:grid-cols-2" aria-label={t('create.examples')}>
          <li>
            <button
              type="button"
              onClick={() => setChoice({ kind: 'blank' })}
              className="grid h-full w-full gap-1 rounded-lg border border-dashed p-4 text-left transition-colors hover:border-primary/60 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
            >
              <span className="flex items-center gap-2 font-medium">
                <FilePlus2 className="size-4" aria-hidden="true" />
                {t('create.blank')}
              </span>
              <span className="text-sm text-muted-foreground">{t('create.blankDescription')}</span>
            </button>
          </li>
          {AUTOMATION_EXAMPLES.map((example) => {
            const enabled = available(example)
            return (
              <li key={example.key}>
                <button
                  type="button"
                  disabled={!enabled}
                  onClick={() => setChoice({ kind: 'example', example })}
                  className="grid h-full w-full gap-1 rounded-lg border bg-card p-4 text-left transition-colors hover:border-primary/60 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-60"
                >
                  <span className="flex items-center gap-2 font-medium">
                    <Sparkles className="size-4 text-primary" aria-hidden="true" />
                    {t(`examples.${example.key}.name`)}
                  </span>
                  <span className="text-sm text-muted-foreground">{t(`examples.${example.key}.description`)}</span>
                  <span className="text-xs text-muted-foreground">
                    {labels.recordTypeLabel(example.recordType)} · {labels.triggerLabel(example.trigger)}
                  </span>
                  {!enabled && <span className="text-xs text-warning">{t('create.unavailable')}</span>}
                </button>
              </li>
            )
          })}
        </ul>
      </div>
    )
  }

  return (
    <ReferenceOptions kind="pipeline_stage" context={STAGE_CONTEXT}>
      {({ options }) => (
        <CreateForm
          choice={choice}
          catalog={catalog}
          recordTypes={recordTypes}
          stageOptions={options}
          onBack={() => setChoice(null)}
          onDone={onDone}
        />
      )}
    </ReferenceOptions>
  )
}

function CreateForm({
  choice,
  catalog,
  recordTypes,
  stageOptions,
  onBack,
  onDone,
}: {
  choice: Choice
  catalog: AutomationCatalog
  recordTypes: string[]
  stageOptions: ReferenceOption[]
  onBack: () => void
  onDone: () => void
}) {
  const { t } = useTranslation('automations')
  const { me } = useSession()
  const navigate = useNavigate()
  const cache = useAutomationCache()
  const labels = useAutomationLabels(catalog)
  const nameId = useId()
  const descriptionId = useId()
  const typeId = useId()
  const triggerId = useId()
  const example = choice.kind === 'example' ? choice.example : null
  const [name, setName] = useState(() => (example ? t(`examples.${example.key}.name`) : ''))
  const [description, setDescription] = useState(() => (example ? t(`examples.${example.key}.description`) : ''))
  const [recordType, setRecordType] = useState(example?.recordType ?? recordTypes[0] ?? '')
  const triggers = catalog.triggers.filter((entry) => supportsRecordType(entry, recordType))
  const [trigger, setTrigger] = useState(example?.trigger ?? 'record.created')
  const effectiveTrigger = triggers.some((entry) => entry.key === trigger) ? trigger : (triggers[0]?.key ?? '')

  const create = useMutation({
    mutationFn: (payload: CreateAutomationPayload) => automationsApi.create(payload),
    onSuccess: async (automation) => {
      await cache.store(automation)
      onDone()
      void navigate(automationPath(automation.id))
    },
  })

  const submit = (event: FormEvent) => {
    event.preventDefault()
    const stages = stageOptions.map((option) => ({ id: option.value, name: option.label }))
    const base: CreateAutomationPayload = example
      ? example.build({ t, userId: me.user.id, stages })
      : { name, record_type: recordType, trigger: { type: effectiveTrigger }, definition: { steps: [] } }
    create.mutate({ ...base, name: name.trim(), description: description.trim() || null })
  }

  const report = create.error ? reportFromError(create.error, { steps: [] }) : null
  const issues = report ? [...report.details, ...report.trigger, ...report.general, ...Object.values(report.steps).flat()] : []

  return (
    <form onSubmit={submit} className="grid gap-4">
      <div className="grid gap-1.5">
        <Label htmlFor={nameId}>{t('create.name')}</Label>
        <Input placeholder={t('placeholders.name')} id={nameId} value={name} onChange={(event) => setName(event.target.value)} maxLength={120} required autoFocus />
      </div>
      <div className="grid gap-1.5">
        <Label htmlFor={descriptionId}>{t('create.descriptionLabel')}</Label>
        <Textarea placeholder={t('placeholders.description')} id={descriptionId} value={description} onChange={(event) => setDescription(event.target.value)} rows={2} maxLength={1000} />
      </div>
      {example ? (
        <p className="text-sm text-muted-foreground">
          {t('create.exampleSummary', {
            type: labels.recordTypeLabel(example.recordType),
            trigger: labels.triggerLabel(example.trigger),
          })}
        </p>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="grid gap-1.5">
            <Label htmlFor={typeId}>{t('create.recordType')}</Label>
            <NativeSelect id={typeId} value={recordType} onChange={(event) => setRecordType(event.target.value)} className="w-full">
              {recordTypes.map((item) => (
                <option key={item} value={item}>
                  {labels.recordTypeLabel(item)}
                </option>
              ))}
            </NativeSelect>
            <p className="text-xs text-muted-foreground">{t('create.recordTypeHint')}</p>
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor={triggerId}>{t('create.trigger')}</Label>
            <NativeSelect id={triggerId} value={effectiveTrigger} onChange={(event) => setTrigger(event.target.value)} className="w-full">
              {triggers.map((entry) => (
                <option key={entry.key} value={entry.key}>
                  {labels.triggerLabel(entry.key, entry)}
                </option>
              ))}
            </NativeSelect>
          </div>
        </div>
      )}
      {create.isError && (
        <div className="grid gap-1 rounded-md border border-destructive/50 p-3">
          <p className="text-sm text-destructive">{t('create.failed')}</p>
          <IssueList issues={issues} showPath />
        </div>
      )}
      <DialogFooter className="gap-2">
        <Button type="button" variant="ghost" onClick={onBack}>
          <ArrowLeft aria-hidden="true" />
          {t('create.back')}
        </Button>
        <Button type="submit" disabled={create.isPending || !name.trim() || !recordType || (!example && !effectiveTrigger)}>
          {create.isPending ? t('create.creating') : t('create.submit')}
        </Button>
      </DialogFooter>
    </form>
  )
}
