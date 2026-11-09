import { Lock, Users } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Slot } from '@/kernel/modules/Slot'
import { chordText, useActionKeys, useKeybindingProps } from '@/kernel/keybindings'
import { type ComposerSubject } from '@/kernel/modules/slots'
import { ContractComponent, NativeSelect, type ContractComponentProps } from '@/sdk'
import { Textarea } from '@/components/ui/textarea'
import type { NoteVisibility } from '../api'
import { Markdown } from '@/components/markdown'
import { Field } from '@/components/ui/field'

export const MAX_NOTE_LENGTH = 50_000

function PlainBody({ onValueChange, ...props }: ContractComponentProps<'notifications.MentionInput'>) {
  void onValueChange
  return <Textarea {...props} />
}

export interface NoteEditorValue {
  body: string
  visibility: NoteVisibility
}

export interface NoteEditorProps {
  initial?: NoteEditorValue
  submitLabel: string
  pending?: boolean
  allowVisibility?: boolean
  subject?: ComposerSubject
  onSubmit: (value: NoteEditorValue) => void | Promise<void>
  onCancel?: () => void
}

export function NoteEditor({
  initial,
  submitLabel,
  pending = false,
  allowVisibility = true,
  subject,
  onSubmit,
  onCancel,
}: NoteEditorProps) {
  const { t } = useTranslation('notes')
  const id = useId()
  const [body, setBody] = useState(initial?.body ?? '')
  const [visibility, setVisibility] = useState<NoteVisibility>(initial?.visibility ?? 'team')
  const [preview, setPreview] = useState(false)
  const empty = body.trim() === ''
  const tooLong = body.length > MAX_NOTE_LENGTH

  const submit = async () => {
    if (empty || tooLong) return
    try {
      await onSubmit({ body: body.trim(), visibility })
    } catch {
      return
    }
    if (!initial) {
      setBody('')
      setPreview(false)
    }
  }
  const submitKeys = useKeybindingProps<HTMLTextAreaElement>({ 'notes.save': () => void submit() })
  const submitChord = useActionKeys('notes.save')[0]

  return (
    <form
      className="grid gap-2"
      onSubmit={(event) => {
        event.preventDefault()
        void submit()
      }}
    >
      <Field
        id={`${id}-body`}
        label={t('editor.label')}
        labelAction={
          <div className="flex flex-wrap items-center justify-end gap-1">
          {!preview && <Slot name="composer.tools" props={{ context: 'note', subject, value: body, onChange: setBody }} />}
          <Button
          type="button"
          variant="ghost"
          size="sm"
          aria-pressed={preview}
          onClick={() => setPreview((current) => !current)}
        >
          {t(preview ? 'editor.write' : 'editor.preview')}
          </Button>
          </div>
        }
      >
      {preview ? (
        <div className="min-h-24 rounded-md border px-3 py-2" aria-label={t('editor.previewLabel')}>
          {empty ? (
            <p className="text-sm text-muted-foreground">{t('editor.previewEmpty')}</p>
          ) : (
            <Markdown source={body} />
          )}
        </div>
      ) : (
        <ContractComponent
          name="notifications.MentionInput"
          fallback={PlainBody}
          id={`${id}-body`}
          rows={4}
          value={body}
          placeholder={t('editor.placeholder')}
          aria-invalid={tooLong ? true : undefined}
          aria-describedby={`${id}-hint`}
          onChange={(event) => setBody(event.target.value)}
          onKeyDown={submitKeys.onKeyDown}
        />
      )}
      </Field>
      <p id={`${id}-hint`} className={tooLong ? 'text-xs text-destructive' : 'text-xs text-muted-foreground'}>
        {tooLong ? t('editor.tooLong') : submitChord ? t('editor.hintKeys', { keys: chordText(submitChord) }) : t('editor.hint')}
      </p>
      <div className="flex flex-wrap items-center justify-between gap-2">
        {allowVisibility ? (
          <div className="flex items-center gap-2">
            {visibility === 'private' ? (
              <Lock className="size-4 text-muted-foreground" aria-hidden="true" />
            ) : (
              <Users className="size-4 text-muted-foreground" aria-hidden="true" />
            )}
            <NativeSelect
              aria-label={t('editor.visibility')}
              value={visibility}
              onChange={(event) => setVisibility(event.target.value as NoteVisibility)}
            >
              <option value="team">{t('visibility.team')}</option>
              <option value="private">{t('visibility.private')}</option>
            </NativeSelect>
          </div>
        ) : (
          <span />
        )}
        <div className="flex gap-2">
          {onCancel && (
            <Button type="button" variant="outline" size="sm" onClick={onCancel} disabled={pending}>
              {t('editor.cancel')}
            </Button>
          )}
          <Button type="submit" size="sm" disabled={pending || empty || tooLong}>
            {submitLabel}
          </Button>
        </div>
      </div>
    </form>
  )
}
