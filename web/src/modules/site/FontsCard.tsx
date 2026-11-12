import { useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Trash2, Upload } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { NativeSelect } from '@/components/ui/native-select'
import { notifyError, notifySuccess } from '@/components/feedback'
import { uploadFile } from '@/sdk'
import { designApi, type DesignRecord } from './designApi'
import { fontKeys, parseFont, useFonts, type FontContent } from './fonts'

function slugify(text: string) {
  return text
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 32)
}

const WEIGHTS = [100, 200, 300, 400, 500, 600, 700, 800, 900]

function FontRow({ record, disabled, onChanged }: { record: DesignRecord; disabled: boolean; onChanged: () => Promise<void> }) {
  const { t } = useTranslation('site')
  const [busy, setBusy] = useState(false)
  const [weight, setWeight] = useState(400)
  const [style, setStyle] = useState<'normal' | 'italic'>('normal')
  const content = parseFont(record.draft_content ?? record.content)

  async function save(next: FontContent) {
    const saved = await designApi.save(record, { draft_content: JSON.stringify(next) })
    await designApi.publish(saved)
    await onChanged()
  }

  async function upload(file: File) {
    setBusy(true)
    try {
      const uploaded = await uploadFile({ file, target: { recordType: 'site.design', recordId: record.id } })
      const latest = await designApi.load(record.id)
      const current = parseFont(latest.draft_content ?? latest.content)
      const saved = await designApi.save(latest, { draft_content: JSON.stringify({ ...current, faces: [...current.faces, { file: uploaded.file.id, weight, style, name: file.name }] }) })
      await designApi.publish(saved)
      await onChanged()
      notifySuccess(t('fonts.uploaded'))
    } catch (error) {
      notifyError(error)
    } finally {
      setBusy(false)
    }
  }

  return (
    <li className="grid gap-3 rounded-lg border p-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <strong style={{ fontFamily: `"${content.family}", ${content.fallback}` }} className="text-lg">
          {content.family} · {t('stylesEditor.sample')}
        </strong>
        <Button
          type="button"
          size="icon"
          variant="ghost"
          aria-label={t('fonts.remove')}
          title={t('fonts.remove')}
          disabled={disabled || busy}
          onClick={() => {
            setBusy(true)
            designApi
              .remove(record)
              .then(onChanged)
              .catch(notifyError)
              .finally(() => setBusy(false))
          }}
        >
          <Trash2 aria-hidden />
        </Button>
      </div>
      {content.faces.length > 0 && (
        <ul className="grid gap-1 text-sm">
          {content.faces.map((face, index) => (
            <li key={face.file} className="flex items-center justify-between gap-2">
              <span>
                {face.name ?? face.file.slice(0, 8)} · {face.weight} · {t(`fonts.style_${face.style}`)}
              </span>
              <Button type="button" size="sm" variant="ghost" disabled={disabled || busy} onClick={() => void save({ ...content, faces: content.faces.filter((_, position) => position !== index) }).catch(notifyError)}>
                {t('fonts.removeFile')}
              </Button>
            </li>
          ))}
        </ul>
      )}
      <div className="grid gap-3 md:grid-cols-3">
        <Field label={t('fonts.weight')}>
          {(control) => (
            <NativeSelect {...control} className="w-full" value={String(weight)} onValueChange={(value) => setWeight(Number(value))}>
              {WEIGHTS.map((value) => (
                <option key={value} value={value}>
                  {value}
                </option>
              ))}
            </NativeSelect>
          )}
        </Field>
        <Field label={t('fonts.style')}>
          {(control) => (
            <NativeSelect {...control} className="w-full" value={style} onValueChange={(value) => setStyle(value as 'normal' | 'italic')}>
              <option value="normal">{t('fonts.style_normal')}</option>
              <option value="italic">{t('fonts.style_italic')}</option>
            </NativeSelect>
          )}
        </Field>
        <Field label={t('fonts.file')} description={t('fonts.fileHelp')}>
          {(control) => (
            <label className="inline-flex h-9 cursor-pointer items-center gap-2 rounded-md border px-3 text-sm">
              <Upload aria-hidden size={16} />
              {busy ? t('fonts.uploading') : t('fonts.choose')}
              <input
                {...control}
                type="file"
                accept=".woff2,.woff,font/woff2,font/woff"
                className="sr-only"
                disabled={disabled || busy}
                onChange={(event) => {
                  const file = event.target.files?.[0]
                  event.target.value = ''
                  if (file) void upload(file)
                }}
              />
            </label>
          )}
        </Field>
      </div>
    </li>
  )
}

export function FontsCard({ disabled }: { disabled: boolean }) {
  const { t } = useTranslation('site')
  const client = useQueryClient()
  const fonts = useFonts()
  const [family, setFamily] = useState('')
  const [fallback, setFallback] = useState<FontContent['fallback']>('sans-serif')
  const changed = async () => {
    await client.invalidateQueries({ queryKey: fontKeys.list })
    await client.invalidateQueries({ queryKey: ['site', 'appearance-catalog'] })
  }
  async function add() {
    const name = family.trim()
    if (!name) return
    try {
      const record = await designApi.create({ target: `font:${slugify(name) || `font-${Date.now()}`}`, kind: 'font', name, draft_content: JSON.stringify({ family: name, fallback, faces: [] }) })
      await designApi.publish(record)
      setFamily('')
      await changed()
    } catch (error) {
      notifyError(error)
    }
  }
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('fonts.title')}</CardTitle>
      </CardHeader>
      <CardContent className="grid gap-4">
        <p className="text-sm text-muted-foreground">{t('fonts.help')}</p>
        <ul className="grid gap-3">
          {(fonts.data ?? []).map((record) => (
            <FontRow key={record.id} record={record} disabled={disabled} onChanged={changed} />
          ))}
        </ul>
        <form
          className="grid items-end gap-3 md:grid-cols-[minmax(0,1fr)_12rem_auto]"
          onSubmit={(event) => {
            event.preventDefault()
            void add()
          }}
        >
          <Field label={t('fonts.family')}>{(control) => <Input {...control} value={family} maxLength={60} placeholder="Lora" onChange={(event) => setFamily(event.target.value)} />}</Field>
          <Field label={t('fonts.fallback')}>
            {(control) => (
              <NativeSelect {...control} className="w-full" value={fallback} onValueChange={(value) => setFallback(value as FontContent['fallback'])}>
                <option value="sans-serif">{t('fonts.fallback_sans')}</option>
                <option value="serif">{t('fonts.fallback_serif')}</option>
                <option value="monospace">{t('fonts.fallback_mono')}</option>
              </NativeSelect>
            )}
          </Field>
          <Button type="submit" variant="outline" disabled={disabled || !family.trim()}>
            {t('fonts.add')}
          </Button>
        </form>
      </CardContent>
    </Card>
  )
}
