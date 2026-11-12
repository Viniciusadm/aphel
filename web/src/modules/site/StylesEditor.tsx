import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { HexColorInput } from './HexColorInput'
import { NativeSelect } from '@/components/ui/native-select'
import { Plus, RotateCcw, Trash2 } from 'lucide-react'

import { mergeStyles } from './stylesDocument'
import { FontsCard } from './FontsCard'
import { parseFont, useFonts } from './fonts'
import { Textarea } from '@/components/ui/textarea'

type Values = Record<string, unknown>
export type StylesDocument = Record<string, unknown>
interface PaletteEntry { slug: string; name?: string; light?: string; dark?: string }
interface FamilyEntry { slug: string; name?: string; stack?: string }
export interface FontStack { key: string; stack: string }

const REQUIRED = ['base', 'surface', 'contrast', 'muted', 'border', 'accent', 'link', 'on-accent']
const CONTENT_WIDTHS = [['40rem', 'content_40'], ['46rem', 'content_46'], ['52rem', 'content_52'], ['60rem', 'content_60']] as const
const WIDE_WIDTHS = [['60rem', 'wide_60'], ['68rem', 'wide_68'], ['76rem', 'wide_76'], ['84rem', 'wide_84']] as const
const BODY_SIZES = ['sm', 'md', 'lg', 'xl'] as const
const HEADING_SIZES = ['3xl', '4xl', '5xl'] as const

function luminance(hex: string) {
  const channels = [1, 3, 5].map((start) => parseInt(hex.slice(start, start + 2), 16) / 255).map((value) => (value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4))
  return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2]
}

function contrast(a: string, b: string) {
  const [x, y] = [luminance(a), luminance(b)]
  return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05)
}

function slugify(text: string) {
  return text
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .replace(/^[^a-z]+/, '')
    .slice(0, 32)
}

interface StylesEditorProps {
  base: StylesDocument
  overrides: StylesDocument
  fontStacks: FontStack[]
  disabled: boolean
  onChange: (overrides: StylesDocument | undefined) => void
}

export function StylesEditor({ base, overrides, fontStacks, disabled, onChange }: StylesEditorProps) {
  const { t } = useTranslation('site')
  const merged = mergeStyles(base, overrides) as StylesDocument
  const palette = (merged.palette as PaletteEntry[] | undefined) ?? []
  const families = (merged.families as FamilyEntry[] | undefined) ?? []
  const elements = (merged.elements as Record<string, Values> | undefined) ?? {}
  const spacing = (merged.spacing as Values | undefined) ?? {}
  const shape = (merged.shape as Values | undefined) ?? {}
  const fonts = t('stylesEditor.fonts', { returnObjects: true }) as Record<string, string>
  const uploaded = useFonts()
  const uploadedOptions = (uploaded.data ?? []).map((record) => ({ value: `font:${record.target.slice(5)}`, label: parseFont(record.draft_content ?? record.content).family || record.name || record.target }))
  const update = (patch: StylesDocument) => {
    const next = mergeStyles(overrides, patch) as StylesDocument
    onChange(Object.keys(next).length ? next : undefined)
  }
  const setColor = (slug: string, key: 'light' | 'dark', value: string) => update({ palette: [{ slug, [key]: value }] })
  const setElement = (element: string, changes: Values) => update({ elements: { [element]: changes } })
  const removeColor = (slug: string) => {
    const own = ((overrides.palette as PaletteEntry[] | undefined) ?? []).filter((entry) => entry.slug !== slug)
    const next = { ...overrides, palette: own }
    if (own.length === 0) delete (next as Values).palette
    onChange(Object.keys(next).length ? next : undefined)
  }
  const addColor = () => {
    const name = t('stylesEditor.newColor')
    let slug = slugify(name) || 'color'
    let counter = 2
    while (palette.some((entry) => entry.slug === slug)) slug = `${slugify(name) || 'color'}-${counter++}`
    update({ palette: [{ slug, name, light: '#888888', dark: '#888888' }] })
  }
  const pick = (slug: string, mode: 'light' | 'dark') => {
    const entry = palette.find((item) => item.slug === slug)
    return (mode === 'dark' ? entry?.dark : undefined) ?? entry?.light ?? '#000000'
  }
  const problems = (['light', 'dark'] as const).flatMap((mode) =>
    [['contrast', 'base'], ['muted', 'base'], ['link', 'base'], ['on-accent', 'accent']]
      .filter(([front, back]) => contrast(pick(front, mode), pick(back, mode)) < 4.5)
      .map(([front, back]) => t('stylesEditor.contrastProblem', { mode: t(`stylesEditor.mode_${mode}`), front: palette.find((entry) => entry.slug === front)?.name ?? front, back: palette.find((entry) => entry.slug === back)?.name ?? back })),
  )
  const select = (label: string, value: string, options: { value: string; label: string }[], onSelect: (value: string) => void) => (
    <Field label={label}>
      {(control) => (
        <NativeSelect {...control} className="w-full" disabled={disabled} value={value} onValueChange={onSelect}>
          {options.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </NativeSelect>
      )}
    </Field>
  )
  return (
    <div className="grid gap-6">
      <Card>
        <CardHeader className="flex-row items-center justify-between gap-4">
          <CardTitle>{t('stylesEditor.colors')}</CardTitle>
          {Object.keys(overrides).length > 0 && (
            <Button size="sm" variant="ghost" disabled={disabled} onClick={() => onChange(undefined)}>
              <RotateCcw aria-hidden />
              {t('stylesEditor.reset')}
            </Button>
          )}
        </CardHeader>
        <CardContent className="grid gap-5">
          {select(t('stylesEditor.mode'), String(merged.mode ?? 'system'), (['system', 'light', 'dark'] as const).map((value) => ({ value, label: t(`stylesEditor.mode_${value}`) })), (mode) => update({ mode }))}
          <div className="grid gap-2">
            <div className="grid grid-cols-[minmax(0,1fr)_9.5rem_9.5rem_2.25rem] gap-3 text-xs font-medium text-muted-foreground" aria-hidden="true">
              <span>{t('stylesEditor.colorName')}</span>
              <span>{t('stylesEditor.mode_light')}</span>
              <span>{t('stylesEditor.mode_dark')}</span>
            </div>
            {palette.map((entry) => (
              <div key={entry.slug} className="grid grid-cols-[minmax(0,1fr)_9.5rem_9.5rem_2.25rem] items-center gap-3">
                {REQUIRED.includes(entry.slug) ? (
                  <span className="truncate text-sm font-medium">{entry.name ?? entry.slug}</span>
                ) : (
                  <Input aria-label={t('stylesEditor.colorName')} value={entry.name ?? ''} maxLength={40} disabled={disabled} onChange={(event) => update({ palette: [{ slug: entry.slug, name: event.target.value }] })} />
                )}
                {(['light', 'dark'] as const).map((mode) => (
                  <HexColorInput key={mode} label={`${entry.name ?? entry.slug} · ${t(`stylesEditor.mode_${mode}`)}`} value={((mode === 'dark' ? entry.dark : undefined) ?? entry.light ?? '#000000').toLowerCase()} disabled={disabled} onChange={(color) => setColor(entry.slug, mode, color)} />
                ))}
                {!REQUIRED.includes(entry.slug) && (
                  <Button size="icon" variant="ghost" aria-label={t('stylesEditor.removeColor')} title={t('stylesEditor.removeColor')} disabled={disabled} onClick={() => removeColor(entry.slug)}>
                    <Trash2 aria-hidden />
                  </Button>
                )}
              </div>
            ))}
          </div>
          <Button variant="outline" size="sm" className="justify-self-start" disabled={disabled || palette.length >= 24} onClick={addColor}>
            <Plus aria-hidden />
            {t('stylesEditor.addColor')}
          </Button>
          {problems.length > 0 && (
            <ul role="status" className="grid gap-1 text-sm text-destructive">
              {problems.map((problem) => (
                <li key={problem}>{problem}</li>
              ))}
            </ul>
          )}
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>{t('stylesEditor.typography')}</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-5 md:grid-cols-2">
          {families
            .filter((family) => family.slug !== 'mono')
            .map((family) => (
              <div key={family.slug} className="grid gap-2">
                {select(family.name ?? family.slug, family.stack ?? 'sans', [...uploadedOptions, ...fontStacks.map((stack) => ({ value: stack.key, label: fonts[stack.key] ?? stack.key }))], (stack) => update({ families: [{ slug: family.slug, stack }] }))}
                <p className="truncate text-lg text-muted-foreground" style={{ fontFamily: family.stack?.startsWith('font:') ? `"${uploadedOptions.find((option) => option.value === family.stack)?.label ?? ''}"` : fontStacks.find((stack) => stack.key === family.stack)?.stack }}>
                  {t('stylesEditor.sample')}
                </p>
              </div>
            ))}
          {select(t('stylesEditor.bodySize'), String(elements.body?.font_size ?? 'md'), BODY_SIZES.map((value) => ({ value, label: t(`stylesEditor.size_${value}`) })), (font_size) => setElement('body', { font_size }))}
          {select(t('stylesEditor.lineHeight'), String(elements.body?.line_height ?? 'normal'), (['snug', 'normal', 'relaxed', 'loose'] as const).map((value) => ({ value, label: t(`stylesEditor.line_${value}`) })), (line_height) => setElement('body', { line_height }))}
          {select(t('stylesEditor.headingSize'), String(elements.h1?.font_size ?? '5xl'), HEADING_SIZES.map((value) => ({ value, label: t(`stylesEditor.heading_${value}`) })), (font_size) => setElement('h1', { font_size }))}
          {select(t('stylesEditor.headingWeight'), String(elements.heading?.font_weight ?? '700'), (['400', '500', '600', '700', '800', '900'] as const).map((value) => ({ value, label: t(`stylesEditor.weight_${value}`) })), (font_weight) => setElement('heading', { font_weight }))}
          {select(t('stylesEditor.linkDecoration'), String(elements.link?.decoration ?? 'underline'), (['underline', 'none'] as const).map((value) => ({ value, label: t(`stylesEditor.decoration_${value}`) })), (decoration) => setElement('link', { decoration }))}
          {select(t('stylesEditor.buttonShape'), String(elements.button?.radius ?? 'full'), (['none', 'sm', 'md', 'full'] as const).map((value) => ({ value, label: t(`stylesEditor.radius_${value}`) })), (radius) => setElement('button', { radius }))}
        </CardContent>
      </Card>
      <FontsCard disabled={disabled} />
      <Card>
        <CardHeader>
          <CardTitle>{t('stylesEditor.css')}</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-3">
          <Field label={t('stylesEditor.cssLabel')} description={t('stylesEditor.cssHelp')}>
            {(control) => <Textarea {...control} rows={10} spellCheck={false} className="font-mono text-sm" value={String(overrides.css ?? '')} disabled={disabled} onChange={(event) => { const next = { ...overrides }; if (event.target.value) next.css = event.target.value; else delete next.css; onChange(Object.keys(next).length ? next : undefined) }} />}
          </Field>
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>{t('stylesEditor.layout')}</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-5 md:grid-cols-2">
          {select(t('stylesEditor.radius'), String(shape.radius ?? 'soft'), (['none', 'soft', 'round'] as const).map((value) => ({ value, label: t(`stylesEditor.shape_${value}`) })), (radius) => update({ shape: { radius } }))}
          {select(t('stylesEditor.shadow'), String(shape.shadow ?? 'none'), (['none', 'soft', 'strong'] as const).map((value) => ({ value, label: t(`stylesEditor.shadow_${value}`) })), (shadow) => update({ shape: { shadow } }))}
          {select(t('stylesEditor.contentWidth'), String(spacing.content ?? '46rem'), CONTENT_WIDTHS.map(([value, key]) => ({ value, label: t(`stylesEditor.${key}`) })), (content) => update({ spacing: { content } }))}
          {select(t('stylesEditor.wideWidth'), String(spacing.wide ?? '68rem'), WIDE_WIDTHS.map(([value, key]) => ({ value, label: t(`stylesEditor.${key}`) })), (wide) => update({ spacing: { wide } }))}
        </CardContent>
      </Card>
    </div>
  )
}
