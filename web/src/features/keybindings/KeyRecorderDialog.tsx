import { useRef, useState, type KeyboardEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field } from '@/components/ui/field'
import { ToggleField } from '@/components/ui/toggle-field'
import {
  KeybindingKeys,
  SEQUENCE_TIMEOUT_MS,
  strokeFromEvent,
  strokeHasCommandModifier,
  strokeIsFunctionKey,
  type KeybindingAction,
  type KeybindingConflict,
} from '@/kernel/keybindings'
import { cn } from '@/lib/utils'

export interface RecordedChoice {
  chord: string
  append: boolean
  resolveConflicts: boolean
}

export interface KeyRecorderDialogProps {
  action: KeybindingAction | null
  currentKeys: string[]
  pending: boolean
  conflictsFor: (chord: string) => KeybindingConflict[]
  describeConflict: (conflict: KeybindingConflict) => string
  onSave: (choice: RecordedChoice) => void
  onOpenChange: (open: boolean) => void
}

const IGNORED_KEYS = new Set(['Escape', 'Tab'])

export function KeyRecorderDialog(props: KeyRecorderDialogProps) {
  return (
    <Dialog open={props.action !== null} onOpenChange={props.onOpenChange}>
      {props.action && <RecorderContent key={props.action.id} {...props} action={props.action} />}
    </Dialog>
  )
}

function RecorderContent({
  action,
  currentKeys,
  pending,
  conflictsFor,
  describeConflict,
  onSave,
  onOpenChange,
}: KeyRecorderDialogProps & { action: KeybindingAction }) {
  const { t } = useTranslation('keybindings')
  const [steps, setSteps] = useState<string[]>([])
  const [append, setAppend] = useState(false)
  const lastStroke = useRef(0)

  const chord = steps.join(' ')
  const conflicts = chord ? conflictsFor(chord) : []
  const errors = conflicts.filter((conflict) => conflict.severity === 'error')
  const warnings = conflicts.filter((conflict) => conflict.severity === 'warning')

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (IGNORED_KEYS.has(event.key) && !event.ctrlKey && !event.metaKey && !event.altKey) return
    const stroke = strokeFromEvent(event.nativeEvent)
    if (!stroke) return
    event.preventDefault()
    event.stopPropagation()
    if (event.repeat) return
    const now = Date.now()
    const previous = steps.at(-1)
    const extendable =
      steps.length === 1 &&
      previous !== undefined &&
      !strokeHasCommandModifier(previous) &&
      !strokeIsFunctionKey(previous) &&
      now - lastStroke.current <= SEQUENCE_TIMEOUT_MS
    lastStroke.current = now
    setSteps(extendable ? [...steps, stroke] : [stroke])
  }

  return (
    <DialogContent size="md">
      <DialogHeader>
        <DialogTitle>{t('recorder.title', { action: action.label })}</DialogTitle>
        <DialogDescription>{t('recorder.description')}</DialogDescription>
      </DialogHeader>
      <Field label={t('recorder.label')} description={t('recorder.hint')}>
        {(control) => (
          <div
            {...control}
            role="textbox"
            aria-readonly="true"
            tabIndex={0}
            autoFocus
            onKeyDown={onKeyDown}
            className={cn(
              'flex min-h-14 items-center justify-center rounded-md border border-dashed px-3 py-2 text-sm outline-none focus-visible:border-primary-solid focus-visible:ring-2 focus-visible:ring-ring',
              errors.length > 0 && 'border-destructive',
            )}
          >
            {chord ? <KeybindingKeys keys={[chord]} size="md" /> : <span className="text-muted-foreground">{t('recorder.placeholder')}</span>}
          </div>
        )}
      </Field>
      {currentKeys.length > 0 && (
        <ToggleField
          label={t('recorder.append')}
          description={t('recorder.appendHint')}
          checked={append}
          onCheckedChange={setAppend}
        />
      )}
      {errors.length > 0 && (
        <div role="alert" className="grid gap-1 rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm">
          {errors.map((conflict) => (
            <p key={`${conflict.kind}:${conflict.otherId}`}>{describeConflict(conflict)}</p>
          ))}
          <p className="text-muted-foreground">{t('recorder.replaceHint')}</p>
        </div>
      )}
      {warnings.length > 0 && (
        <div role="status" className="grid gap-1 rounded-md border border-warning/30 bg-warning/5 px-3 py-2 text-sm">
          {warnings.map((conflict) => (
            <p key={`${conflict.kind}:${conflict.otherId ?? conflict.chord}`}>{describeConflict(conflict)}</p>
          ))}
        </div>
      )}
      <DialogFooter>
        <Button variant="ghost" onClick={() => setSteps([])} disabled={!chord}>
          {t('recorder.clear')}
        </Button>
        <Button variant="outline" onClick={() => onOpenChange(false)}>
          {t('recorder.cancel')}
        </Button>
        <Button
          disabled={!chord || pending}
          variant={errors.length > 0 ? 'destructive' : 'default'}
          onClick={() => onSave({ chord, append, resolveConflicts: errors.length > 0 })}
        >
          {errors.length > 0 ? t('recorder.replace') : t('recorder.save')}
        </Button>
      </DialogFooter>
    </DialogContent>
  )
}
