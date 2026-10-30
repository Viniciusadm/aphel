import { useMutation } from '@tanstack/react-query'
import { Loader2, Sparkles } from 'lucide-react'
import { useId, useState } from 'react'
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
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import type { ComposerContext, SlotProps } from '@/kernel/modules/slots'
import { AI_KEY, AI_RECORD_TYPES, aiApi, useAiStatus } from './api'
import { AiErrorMessage, AiFooter } from './AiNotice'

const SHORTCUTS = ['improve', 'formal', 'friendly', 'shorter', 'grammar', 'summarize', 'reply'] as const
const DRAFT_SHORTCUTS = new Set<string>(['improve', 'formal', 'friendly', 'shorter', 'grammar', 'summarize'])
const MAX_INSTRUCTION = 1000

type Shortcut = (typeof SHORTCUTS)[number]

function availableShortcuts(context: ComposerContext, hasDraft: boolean): Shortcut[] {
  return SHORTCUTS.filter((shortcut) => {
    if (shortcut === 'reply') return context !== 'note'
    return hasDraft || !DRAFT_SHORTCUTS.has(shortcut)
  })
}

export function ComposerTool({ context, subject, value, onChange }: SlotProps<'composer.tools'>) {
  const t = useModuleTranslation(AI_KEY)
  const status = useAiStatus()
  const [open, setOpen] = useState(false)
  if (!status.data?.configured || !status.data.features.compose) return null

  return (
    <>
      <Button type="button" variant="ghost" size="sm" onClick={() => setOpen(true)}>
        <Sparkles aria-hidden="true" />
        {t('compose.button')}
      </Button>
      {open && (
        <ComposerDialog
          context={context}
          subject={subject}
          value={value}
          onChange={onChange}
          onOpenChange={setOpen}
        />
      )}
    </>
  )
}

interface ComposerDialogProps extends SlotProps<'composer.tools'> {
  onOpenChange: (open: boolean) => void
}

function ComposerDialog({ context, subject, value, onChange, onOpenChange }: ComposerDialogProps) {
  const t = useModuleTranslation(AI_KEY)
  const id = useId()
  const [instruction, setInstruction] = useState('')
  const subjectSupported = Boolean(subject && AI_RECORD_TYPES.includes(subject.recordType))
  const [useRecord, setUseRecord] = useState(subjectSupported)
  const hasDraft = value.trim().length > 0
  const compose = useMutation({
    mutationFn: (text: string) =>
      aiApi.compose({
        context,
        instruction: text,
        draft: hasDraft ? value : undefined,
        record_type: useRecord && subject ? subject.recordType : undefined,
        record_id: useRecord && subject ? subject.recordId : undefined,
      }),
  })

  const run = (text: string) => {
    const trimmed = text.trim()
    if (trimmed) compose.mutate(trimmed)
  }

  const apply = (mode: 'replace' | 'insert') => {
    if (!compose.data) return
    const suggestion = compose.data.text
    onChange(mode === 'replace' || !hasDraft ? suggestion : `${value.trimEnd()}\n\n${suggestion}`)
    onOpenChange(false)
  }

  return (
    <Dialog open onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>{t('compose.title')}</DialogTitle>
          <DialogDescription>{t(`compose.description.${context}`)}</DialogDescription>
        </DialogHeader>
        <div className="grid gap-3">
          <div className="flex flex-wrap gap-1.5" role="group" aria-label={t('compose.shortcuts')}>
            {availableShortcuts(context, hasDraft).map((shortcut) => (
              <Button
                key={shortcut}
                type="button"
                size="sm"
                variant="outline"
                disabled={compose.isPending}
                onClick={() => {
                  const text = t(`compose.instructions.${shortcut}`)
                  setInstruction(text)
                  run(text)
                }}
              >
                {t(`compose.shortcut.${shortcut}`)}
              </Button>
            ))}
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor={`${id}-instruction`}>{t('compose.instruction')}</Label>
            <Textarea
              id={`${id}-instruction`}
              rows={3}
              maxLength={MAX_INSTRUCTION}
              placeholder={t(hasDraft ? 'compose.placeholderDraft' : 'compose.placeholderEmpty')}
              value={instruction}
              onChange={(event) => setInstruction(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
                  event.preventDefault()
                  run(instruction)
                }
              }}
            />
          </div>
          {subjectSupported && (
            <div className="flex items-center gap-2">
              <Checkbox
                id={`${id}-record`}
                checked={useRecord}
                onCheckedChange={(checked) => setUseRecord(checked === true)}
              />
              <Label htmlFor={`${id}-record`} className="text-sm font-normal">
                {t('compose.useRecord')}
              </Label>
            </div>
          )}
          <p className="text-xs text-muted-foreground">
            {t(hasDraft ? 'compose.sendsDraft' : 'compose.sendsInstruction')}
          </p>
          {compose.isError && <AiErrorMessage error={compose.error} />}
          {compose.data && !compose.isPending && (
            <div className="grid gap-2">
              <p className="text-sm font-medium">{t('compose.preview')}</p>
              <div className="max-h-72 overflow-y-auto rounded-md border bg-muted/40 p-3 text-sm whitespace-pre-wrap">
                {compose.data.text}
              </div>
              <AiFooter provider={compose.data.provider} model={compose.data.model} />
            </div>
          )}
        </div>
        <DialogFooter className="gap-2">
          <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
            {t('compose.cancel')}
          </Button>
          {compose.data && !compose.isPending ? (
            <>
              <Button type="button" variant="outline" disabled={!instruction.trim()} onClick={() => run(instruction)}>
                {t('compose.regenerate')}
              </Button>
              {hasDraft && (
                <Button type="button" variant="outline" onClick={() => apply('insert')}>
                  {t('compose.insert')}
                </Button>
              )}
              <Button type="button" onClick={() => apply('replace')}>
                {t(hasDraft ? 'compose.replace' : 'compose.use')}
              </Button>
            </>
          ) : (
            <Button type="button" disabled={compose.isPending || !instruction.trim()} onClick={() => run(instruction)}>
              {compose.isPending ? <Loader2 className="animate-spin" aria-hidden="true" /> : <Sparkles aria-hidden="true" />}
              {t('compose.generate')}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
