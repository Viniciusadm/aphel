import { Check } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'

type Values = Record<string, unknown>
export interface ThemePreset { key: string; values: Values; styles: Values }
interface Stack { key: string; stack: string }
interface Color { slug: string; light?: string; dark?: string }

const radii: Record<string, string> = { none: '0', soft: '6px', round: '14px' }

function paletteOf(styles: Values) {
  const dark = styles.mode === 'dark'
  const colors = (styles.palette as Color[] | undefined) ?? []
  return (slug: string) => {
    const color = colors.find((entry) => entry.slug === slug)
    return (dark ? color?.dark : undefined) ?? color?.light ?? '#888888'
  }
}

function ThemeThumbnail({ values, styles, fontStacks }: { values: Values; styles: Values; fontStacks: Stack[] }) {
  const color = paletteOf(styles)
  const background = color('base')
  const surface = color('surface')
  const text = color('contrast')
  const muted = color('muted')
  const accent = color('accent')
  const border = color('border')
  const radius = radii[String((styles.shape as Values | undefined)?.radius)] ?? '6px'
  const heading = ((styles.families as { slug: string; stack?: string }[] | undefined) ?? []).find((family) => family.slug === 'heading')?.stack ?? 'sans'
  const images = values.card_image !== 'none'
  const list = values.layout === 'list'
  const featured = values.layout === 'featured'
  const card = (wide = false) => <div style={{ background: list ? 'transparent' : surface, border: list ? 0 : `1px solid ${border}`, borderRadius: radius, overflow: 'hidden', gridColumn: wide ? '1 / -1' : undefined, display: wide ? 'grid' : 'block', gridTemplateColumns: wide ? '3fr 2fr' : undefined, borderBottom: list ? `1px solid ${border}` : undefined }}>
    {images && <div style={{ height: wide ? 30 : 18, background: `linear-gradient(135deg, ${accent}55, ${accent}22)` }} />}
    <div style={{ padding: 5, display: 'grid', gap: 3 }}>
      <div style={{ height: 4, width: '80%', borderRadius: 2, background: text }} />
      <div style={{ height: 3, width: '55%', borderRadius: 2, background: muted, opacity: 0.6 }} />
    </div>
  </div>
  return <div aria-hidden="true" className="overflow-hidden rounded-md border" style={{ background, color: text, height: 112, padding: 10, display: 'grid', gridTemplateRows: 'auto auto 1fr', gap: 6 }}>
    <div className="flex items-center justify-between">
      <span style={{ fontFamily: fontStacks.find((stack) => stack.key === heading)?.stack, fontWeight: 700, fontSize: 15, lineHeight: 1 }}>Aa</span>
      <span className="flex gap-1">{['base', 'surface', 'contrast', 'accent'].map((key) => <span key={key} style={{ width: 9, height: 9, borderRadius: 999, background: color(key), boxShadow: `0 0 0 1px ${border}` }} />)}</span>
    </div>
    <div style={{ height: 2, background: accent, width: 22, borderRadius: 2 }} />
    <div style={{ display: 'grid', gap: 5, gridTemplateColumns: list ? '1fr' : `repeat(${featured ? 3 : Math.min(Number(values.columns) || 2, 3)}, 1fr)`, alignContent: 'start' }}>
      {featured && card(true)}
      {card()}{card()}{!list && !featured && Number(values.columns) > 2 && card()}
    </div>
  </div>
}

export function ThemePresets({ presets, values, active: current, fontStacks, disabled, onApply, labels = {} }: { presets: ThemePreset[]; values: Values; active?: string; fontStacks: Stack[]; disabled: boolean; onApply: (preset: ThemePreset) => void; labels?: Record<string, string> }) {
  const { t } = useTranslation('site')
  const names = t('appearanceEditor.presets', { returnObjects: true }) as Record<string, string>
  const descriptions = t('appearanceEditor.presetDescriptions', { returnObjects: true }) as Record<string, string>
  const active = (preset: ThemePreset) => (current ? current === preset.key && !values.styles : Object.entries(preset.values).every(([key, value]) => values[key] === value))
  return <section className="grid gap-3" aria-labelledby="site-theme-presets">
    <div>
      <h2 id="site-theme-presets" className="text-base font-semibold">{t('appearanceEditor.presetsTitle')}</h2>
      <p className="text-sm text-muted-foreground">{t('appearanceEditor.presetsHelp')}</p>
    </div>
    <div className="grid grid-cols-[repeat(auto-fill,minmax(11rem,1fr))] gap-3">
      {presets.map((preset) => {
        const selected = active(preset)
        return <button key={preset.key} type="button" disabled={disabled} aria-pressed={selected} aria-label={t('appearanceEditor.presetApply', { name: labels[preset.key] ?? names[preset.key] ?? preset.key })} onClick={() => onApply(preset)} className={cn('group grid gap-2 rounded-lg border bg-card p-2.5 text-left text-card-foreground transition-[border-color,box-shadow,transform] hover:-translate-y-0.5 hover:border-primary/50 hover:shadow-md focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-60', selected && 'border-primary ring-1 ring-primary')}>
          <ThemeThumbnail values={preset.values} styles={preset.styles} fontStacks={fontStacks} />
          <span className="flex items-center justify-between gap-2 px-0.5">
            <span className="text-sm font-semibold">{labels[preset.key] ?? names[preset.key] ?? preset.key}</span>
            {selected && <span className="inline-flex items-center gap-1 text-xs font-medium text-primary"><Check className="size-3.5" aria-hidden="true" />{t('appearanceEditor.presetActive')}</span>}
          </span>
          {descriptions[preset.key] && <span className="px-0.5 text-xs leading-snug text-muted-foreground">{descriptions[preset.key]}</span>}
        </button>
      })}
    </div>
  </section>
}
