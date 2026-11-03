import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { useQueryClient } from '@tanstack/react-query'
import { ArrowDown, ArrowUp, Plus, Trash2 } from 'lucide-react'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { notifySuccess } from '@/components/feedback/notify'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { stageGuidanceApi, stageGuidanceKeys, useStageGuidance } from './api'
import type { StageGuidance } from './types'
import { Field } from '@/components/ui/field'

const GUIDANCE_MAX_LENGTH = 20000
const LABEL_MAX_LENGTH = 200
const MAX_ITEMS = 30

export interface StageGuidanceEditorProps {
  stageId: string
}

interface DraftItem {
  uid: string
  id?: string
  key?: string
  label: string
  required: boolean
}

function draftItems(guidance: StageGuidance): DraftItem[] {
  return guidance.items.map((item) => ({
    uid: item.id,
    id: item.id,
    key: item.key,
    label: item.label,
    required: item.required,
  }))
}

export function StageGuidanceEditor({ stageId }: StageGuidanceEditorProps) {
  const guidance = useStageGuidance(stageId)
  if (guidance.isPending) return <Skeleton className="h-32 w-full" />
  if (guidance.isError) return null
  return <GuidanceForm key={guidance.data.updated_at ?? 'new'} stageId={stageId} guidance={guidance.data} />
}

function GuidanceForm({ stageId, guidance }: { stageId: string; guidance: StageGuidance }) {
  const { t } = useTranslation('workflow', { keyPrefix: 'guidance' })
  const queryClient = useQueryClient()
  const [text, setText] = useState(guidance.guidance)
  const [items, setItems] = useState<DraftItem[]>(() => draftItems(guidance))
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const textId = useId()
  const titleId = useId()

  const update = (uid: string, patch: Partial<DraftItem>) =>
    setItems((current) => current.map((item) => (item.uid === uid ? { ...item, ...patch } : item)))

  const move = (index: number, offset: number) =>
    setItems((current) => {
      const target = index + offset
      if (target < 0 || target >= current.length) return current
      const next = [...current]
      ;[next[index], next[target]] = [next[target], next[index]]
      return next
    })

  const save = async () => {
    if (items.some((item) => !item.label.trim())) {
      setError(t('editor.labelRequired'))
      return
    }
    setSaving(true)
    setError(null)
    try {
      const saved = await stageGuidanceApi.save(
        stageId,
        {
          guidance: text,
          items: items.map((item) => ({
            ...(item.id ? { id: item.id } : {}),
            ...(item.key ? { key: item.key } : {}),
            label: item.label.trim(),
            required: item.required,
          })),
        },
        guidance.updated_at ?? null,
      )
      queryClient.setQueryData(stageGuidanceKeys.stage(stageId), saved)
      void queryClient.invalidateQueries({ queryKey: stageGuidanceKeys.all })
      notifySuccess(t('editor.saved'))
    } catch (cause) {
      setError(isApiError(cause) && cause.status === 409 ? t('editor.conflict') : t('editor.error'))
    } finally {
      setSaving(false)
    }
  }

  return (
    <section className="grid min-w-0 gap-4 border-t pt-4" aria-labelledby={titleId}>
      <div className="grid gap-1">
        <h3 id={titleId} className="text-base font-semibold">
          {t('editor.title')}
        </h3>
        <p className="text-sm text-muted-foreground">{t('editor.description')}</p>
      </div>
      <Field label={t('editor.guidance')} className="min-w-0">
        {(control) => <>
          <Textarea placeholder={t('placeholders.guidance')}
          {...control}
          value={text}
          rows={4}
          maxLength={GUIDANCE_MAX_LENGTH}
          aria-describedby={`${textId}-hint`}
          onChange={(event) => setText(event.target.value)}
        />
        <p id={`${textId}-hint`} className="text-xs text-muted-foreground">
          {t('editor.guidanceHint')}
        </p>
        </>}
      </Field>
      <fieldset className="grid min-w-0 gap-2">
        <legend className="mb-1 text-sm font-medium">{t('editor.items')}</legend>
        {items.length === 0 && <p className="text-sm text-muted-foreground">{t('editor.noItems')}</p>}
        <ol className="grid gap-2">
          {items.map((item, index) => {
            const position = index + 1
            return (
              <li key={item.uid} className="grid min-w-0 gap-2 rounded-md border p-2">
                <div className="flex min-w-0 items-center gap-1">
                  <Input placeholder={t('placeholders.checklistItem')}
                    value={item.label}
                    maxLength={LABEL_MAX_LENGTH}
                    aria-label={t('editor.itemLabel', { index: position })}
                    onChange={(event) => update(item.uid, { label: event.target.value })}
                  />
                  <Button
                    type="button"
                    size="icon"
                    variant="ghost"
                    disabled={index === 0}
                    aria-label={t('editor.moveUp', { index: position })}
                    onClick={() => move(index, -1)}
                  >
                    <ArrowUp aria-hidden="true" />
                  </Button>
                  <Button
                    type="button"
                    size="icon"
                    variant="ghost"
                    disabled={index === items.length - 1}
                    aria-label={t('editor.moveDown', { index: position })}
                    onClick={() => move(index, 1)}
                  >
                    <ArrowDown aria-hidden="true" />
                  </Button>
                  <Button
                    type="button"
                    size="icon"
                    variant="ghost"
                    className="text-destructive"
                    aria-label={t('editor.removeItem', { index: position })}
                    onClick={() => setItems((current) => current.filter((entry) => entry.uid !== item.uid))}
                  >
                    <Trash2 aria-hidden="true" />
                  </Button>
                </div>
                <label className="flex items-center gap-2 text-sm">
                  <Checkbox
                    checked={item.required}
                    onCheckedChange={(value) => update(item.uid, { required: value === true })}
                  />
                  {t('editor.required')}
                </label>
              </li>
            )
          })}
        </ol>
        <div>
          <Button
            type="button"
            size="sm"
            variant="outline"
            disabled={items.length >= MAX_ITEMS}
            onClick={() =>
              setItems((current) => [...current, { uid: crypto.randomUUID(), label: '', required: false }])
            }
          >
            <Plus aria-hidden="true" />
            {t('editor.addItem')}
          </Button>
        </div>
      </fieldset>
      {error && (
        <SharedFieldError>
          {error}
        </SharedFieldError>
      )}
      <div className="flex justify-end">
        <Button type="button" variant="secondary" disabled={saving} onClick={() => void save()}>
          {t('editor.save')}
        </Button>
      </div>
    </section>
  )
}
