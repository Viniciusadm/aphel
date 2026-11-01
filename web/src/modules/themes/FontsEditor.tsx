import { NativeSelect } from '@/components/ui/native-select'
import { Trash2, Upload } from 'lucide-react'
import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { MAX_FONT_BYTES, MAX_FONTS, fontFromFile } from './lib/package'
import type { FontFile } from './lib/types'
import { Field } from '@/components/ui/field'

const WEIGHTS = [100, 200, 300, 400, 500, 600, 700, 800, 900]

interface FontsEditorProps {
  fonts: FontFile[]
  onChange: (fonts: FontFile[]) => void
}

export function FontsEditor({ fonts, onChange }: FontsEditorProps) {
  const { t } = useTranslation('themes')
  const fileInput = useRef<HTMLInputElement>(null)
  const [family, setFamily] = useState('')
  const [weight, setWeight] = useState('400')
  const [style, setStyle] = useState<FontFile['style']>('normal')

  const add = async (file: File | undefined) => {
    if (!file) return
    if (file.size > MAX_FONT_BYTES) {
      notifyError(new Error(t('fonts.tooLarge', { size: MAX_FONT_BYTES / 1024 })))
      return
    }
    const name = family.trim() || file.name.replace(/\.woff2$/i, '').replace(/[^A-Za-z0-9 _-]+/g, ' ').trim()
    onChange([...fonts, await fontFromFile(file, name, Number(weight), style)])
    setFamily('')
  }

  return (
    <div className="grid gap-3">
      <p className="text-sm text-muted-foreground">{t('fonts.description', { max: MAX_FONTS })}</p>
      {fonts.length > 0 && (
        <ul className="grid gap-1">
          {fonts.map((font, index) => (
            <li key={`${font.family}:${font.weight}:${font.style}`} className="flex items-center justify-between gap-2 rounded-md border p-2 text-sm">
              <span>
                {font.family} · {font.weight} · {font.style}
              </span>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                aria-label={t('fonts.remove', { family: font.family })}
                onClick={() => onChange(fonts.filter((_, position) => position !== index))}
              >
                <Trash2 aria-hidden="true" />
              </Button>
            </li>
          ))}
        </ul>
      )}
      {fonts.length < MAX_FONTS && (
        <div className="grid gap-3 sm:grid-cols-4">
          <Field label={t('fonts.family')} className="sm:col-span-2">
            {(control) => <Input placeholder={t('placeholders.family')} {...control} value={family} onChange={(event) => setFamily(event.target.value)} />}
          </Field>
          <Field label={t('fonts.weight')}>
            {(fieldControl) => <NativeSelect value={weight} onValueChange={setWeight} id={fieldControl.id}>
                {WEIGHTS.map((option) => (
                  <option key={option} value={String(option)}>
                    {option}
                  </option>
                ))}
              </NativeSelect>}
          </Field>
          <Field label={t('fonts.style')}>
            {(fieldControl) => <NativeSelect value={style} onValueChange={(next) => setStyle(next as FontFile['style'])} id={fieldControl.id}>
                <option value="normal">normal</option>
                <option value="italic">italic</option>
              </NativeSelect>}
          </Field>
          <div className="sm:col-span-4">
            <input
              ref={fileInput}
              type="file"
              accept=".woff2,font/woff2"
              className="sr-only"
              tabIndex={-1}
              aria-label={t('fonts.add')}
              onChange={(event) => {
                const file = event.target.files?.[0]
                event.target.value = ''
                void add(file)
              }}
            />
            <Button type="button" variant="outline" size="sm" onClick={() => fileInput.current?.click()}>
              <Upload aria-hidden="true" />
              {t('fonts.add')}
            </Button>
          </div>
        </div>
      )}
    </div>
  )
}
