import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { NativeSelect } from '@/components/ui/native-select'
import { ToggleField } from '@/components/ui/toggle-field'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { Paintbrush } from 'lucide-react'

import { useSiteStyles, type PaletteToken } from './api'
import { isContainer, type Block, type BlockStyle, type SpaceStep, type StyleDevice } from './blocks'

type Translate = ReturnType<typeof useModuleTranslation>

const STEPS: SpaceStep[] = ['0', '1', '2', '3', '4', '5', '6', '7', '8']
const DEVICES: StyleDevice[] = ['desktop', 'tablet', 'mobile']

function prune(style: BlockStyle): BlockStyle | undefined {
  const clean: Record<string, unknown> = {}
  for (const [key, value] of Object.entries(style)) {
    if (value === undefined || value === '' || (Array.isArray(value) && value.length === 0)) continue
    if (typeof value === 'object' && !Array.isArray(value)) {
      const inner = Object.fromEntries(Object.entries(value as Record<string, unknown>).filter(([, item]) => item !== undefined && item !== ''))
      if (Object.keys(inner).length === 0) continue
      clean[key] = inner
    } else {
      clean[key] = value
    }
  }
  return Object.keys(clean).length ? (clean as BlockStyle) : undefined
}

function Select<T extends string>({ label, value, options, onChange, disabled, inherit }: { label: string; value: T | undefined; options: { value: T; label: string }[]; onChange: (value: T | undefined) => void; disabled?: boolean; inherit: string }) {
  return (
    <Field label={label}>
      {(control) => (
        <NativeSelect {...control} className="w-full" disabled={disabled} value={value ?? ''} onValueChange={(next) => onChange(next ? (next as T) : undefined)}>
          <option value="">{inherit}</option>
          {options.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </NativeSelect>
      )}
    </Field>
  )
}

function ColorChoice({ label, value, palette, onChange, disabled, t }: { label: string; value: string | undefined; palette: PaletteToken[]; onChange: (value: string | undefined) => void; disabled?: boolean; t: Translate }) {
  const custom = value?.startsWith('#') ? value : undefined
  return (
    <Field label={label}>
      {() => (
        <div className="blocks-swatches" role="radiogroup" aria-label={label}>
          <button type="button" role="radio" aria-checked={!value} className="blocks-swatch blocks-swatch-none" title={t('style.none')} aria-label={t('style.none')} disabled={disabled} onClick={() => onChange(undefined)} />
          {palette.map((color) => (
            <button
              key={color.slug}
              type="button"
              role="radio"
              aria-checked={value === color.slug}
              className="blocks-swatch"
              style={{ background: color.light, ['--swatch-dark' as string]: color.dark ?? color.light }}
              title={color.name ?? color.slug}
              aria-label={color.name ?? color.slug}
              disabled={disabled}
              onClick={() => onChange(color.slug)}
            />
          ))}
          <label className="blocks-swatch blocks-swatch-custom" title={t('style.custom')}>
            <span className="sr-only">{t('style.custom')}</span>
            <input type="color" value={custom ?? '#888888'} disabled={disabled} onChange={(event) => onChange(event.target.value)} />
          </label>
        </div>
      )}
    </Field>
  )
}

export function StyleSection({ block, onChange, disabled }: { block: Block; onChange: (block: Block) => void; disabled?: boolean }) {
  const t = useModuleTranslation('site-blocks')
  const styles = useSiteStyles()
  const style = block.style ?? {}
  const palette = styles.data?.palette ?? []
  const variations = styles.data?.blocks?.[block.type]?.variations ?? []
  const inherit = t('style.inherit')
  const set = (changes: Partial<BlockStyle>) => onChange({ ...block, style: prune({ ...style, ...changes }) })
  const steps = STEPS.map((step) => ({ value: step, label: t('style.step', { step }) }))
  const sizeOf = (side: 'top' | 'right' | 'bottom' | 'left') => style.padding?.[side]
  const filled = Object.keys(style).length
  return (
    <details className="blocks-style" open={filled > 0 || undefined}>
      <summary>
        <Paintbrush aria-hidden size={16} />
        {t('style.title')}
        {filled > 0 && <span className="blocks-style-count">{filled}</span>}
      </summary>
      <div className="blocks-style-body">
        {variations.length > 0 && (
          <Select label={t('style.variation')} value={style.variation} inherit={t('style.variationDefault')} disabled={disabled} options={variations.map((variation) => ({ value: variation.slug, label: variation.name ?? variation.slug }))} onChange={(variation) => set({ variation })} />
        )}
        <fieldset className="blocks-style-group">
          <legend>{t('style.colors')}</legend>
          <ColorChoice label={t('style.text')} value={style.text} palette={palette} disabled={disabled} t={t} onChange={(text) => set({ text })} />
          <ColorChoice label={t('style.background')} value={style.background} palette={palette} disabled={disabled} t={t} onChange={(background) => set({ background })} />
          <ColorChoice label={t('style.link')} value={style.link} palette={palette} disabled={disabled} t={t} onChange={(link) => set({ link })} />
          {(styles.data?.gradients?.length ?? 0) > 0 && (
            <Select label={t('style.gradient')} value={style.gradient} inherit={t('style.none')} disabled={disabled} options={(styles.data?.gradients ?? []).map((gradient) => ({ value: gradient.slug, label: gradient.name ?? gradient.slug }))} onChange={(gradient) => set({ gradient })} />
          )}
        </fieldset>
        <fieldset className="blocks-style-group">
          <legend>{t('style.spacing')}</legend>
          <Select label={t('style.paddingVertical')} value={sizeOf('top')} inherit={inherit} options={steps} disabled={disabled} onChange={(step) => set({ padding: { ...style.padding, top: step, bottom: step } })} />
          <Select label={t('style.paddingHorizontal')} value={sizeOf('left')} inherit={inherit} options={steps} disabled={disabled} onChange={(step) => set({ padding: { ...style.padding, left: step, right: step } })} />
          <Select label={t('style.marginTop')} value={style.margin?.top} inherit={inherit} options={steps} disabled={disabled} onChange={(top) => set({ margin: { ...style.margin, top } })} />
          <Select label={t('style.marginBottom')} value={style.margin?.bottom} inherit={inherit} options={steps} disabled={disabled} onChange={(bottom) => set({ margin: { ...style.margin, bottom } })} />
          {isContainer(block) && <Select label={t('style.gap')} value={style.gap} inherit={inherit} options={steps} disabled={disabled} onChange={(gap) => set({ gap })} />}
        </fieldset>
        <fieldset className="blocks-style-group">
          <legend>{t('style.shape')}</legend>
          <Select label={t('style.borderWidth')} value={style.border?.width} inherit={t('style.none')} disabled={disabled} options={(['1', '2', '4', '8'] as const).map((width) => ({ value: width, label: `${width}px` }))} onChange={(width) => set({ border: { ...style.border, width } })} />
          {style.border?.width && (
            <>
              <ColorChoice label={t('style.borderColor')} value={style.border?.color} palette={palette} disabled={disabled} t={t} onChange={(color) => set({ border: { ...style.border, color } })} />
              <Select label={t('style.borderStyle')} value={style.border?.style} inherit={t('style.solid')} disabled={disabled} options={(['dashed', 'dotted'] as const).map((value) => ({ value, label: t(`style.${value}`) }))} onChange={(value) => set({ border: { ...style.border, style: value } })} />
            </>
          )}
          <Select label={t('style.radius')} value={style.radius} inherit={inherit} disabled={disabled} options={(['none', 'sm', 'md', 'lg', 'full'] as const).map((value) => ({ value, label: t(`style.radius_${value}`) }))} onChange={(radius) => set({ radius })} />
          <Select label={t('style.shadow')} value={style.shadow} inherit={inherit} disabled={disabled} options={(['none', 'sm', 'md', 'lg'] as const).map((value) => ({ value, label: t(`style.shadow_${value}`) }))} onChange={(shadow) => set({ shadow })} />
        </fieldset>
        <fieldset className="blocks-style-group">
          <legend>{t('style.typography')}</legend>
          <Select label={t('style.fontSize')} value={style.font_size} inherit={inherit} disabled={disabled} options={(styles.data?.sizes ?? []).map((size) => ({ value: size.slug, label: size.name ?? size.slug }))} onChange={(font_size) => set({ font_size })} />
          <Select label={t('style.fontFamily')} value={style.font_family} inherit={inherit} disabled={disabled} options={(styles.data?.families ?? []).map((family) => ({ value: family.slug, label: family.name ?? family.slug }))} onChange={(font_family) => set({ font_family })} />
          <Select label={t('style.fontWeight')} value={style.font_weight} inherit={inherit} disabled={disabled} options={(['300', '400', '500', '600', '700', '800', '900'] as const).map((value) => ({ value, label: t(`style.weight_${value}`) }))} onChange={(font_weight) => set({ font_weight })} />
          <Select label={t('style.textAlign')} value={style.text_align} inherit={inherit} disabled={disabled} options={(['left', 'center', 'right'] as const).map((value) => ({ value, label: t(`style.align_${value}`) }))} onChange={(text_align) => set({ text_align })} />
          <Select label={t('style.textTransform')} value={style.text_transform} inherit={inherit} disabled={disabled} options={(['uppercase', 'lowercase', 'capitalize'] as const).map((value) => ({ value, label: t(`style.transform_${value}`) }))} onChange={(text_transform) => set({ text_transform })} />
          <Select label={t('style.lineHeight')} value={style.line_height} inherit={inherit} disabled={disabled} options={(['tight', 'snug', 'normal', 'relaxed', 'loose'] as const).map((value) => ({ value, label: t(`style.line_${value}`) }))} onChange={(line_height) => set({ line_height })} />
          <Select label={t('style.letterSpacing')} value={style.letter_spacing} inherit={inherit} disabled={disabled} options={(['tighter', 'tight', 'normal', 'wide', 'wider'] as const).map((value) => ({ value, label: t(`style.spacing_${value}`) }))} onChange={(letter_spacing) => set({ letter_spacing })} />
        </fieldset>
        <fieldset className="blocks-style-group">
          <legend>{t('style.layout')}</legend>
          <Select label={t('style.width')} value={style.width} inherit={t('style.width_none')} disabled={disabled} options={(['wide', 'full'] as const).map((value) => ({ value, label: t(`style.width_${value}`) }))} onChange={(width) => set({ width })} />
          {DEVICES.map((device) => (
            <ToggleField
              key={device}
              label={t(`style.hide_${device}`)}
              checked={style.hide_on?.includes(device) ?? false}
              disabled={disabled}
              onCheckedChange={(checked) => set({ hide_on: checked ? [...(style.hide_on ?? []), device] : (style.hide_on ?? []).filter((item) => item !== device) })}
            />
          ))}
          <Field label={t('style.classes')} description={t('style.classesHelp')}>
            {(control) => <Input {...control} value={style.classes ?? ''} maxLength={200} spellCheck={false} autoComplete="off" disabled={disabled} onChange={(event) => set({ classes: event.target.value })} />}
          </Field>
        </fieldset>
      </div>
    </details>
  )
}
