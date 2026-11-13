import { useFormContext, useWatch } from 'react-hook-form'
import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/field'
import { HexColorInput } from '@/components/ui/hex-color-input'
import { NativeSelect } from '@/components/ui/native-select'
import { SegmentedControl } from '@/components/ui/segmented-control'
import { Textarea } from '@/components/ui/textarea'
import { cn } from '@/lib/utils'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { FORMS_KEY } from './api'
import type { AppearanceValues, LeadFormValues } from './form'

const THEMES = [
  { value: 'clean', page: '#f3f4f6', surface: '#ffffff', line: '#d1d5db', text: '#111827' },
  { value: 'soft', page: '#f6f1ea', surface: '#fffdf9', line: '#e6ddd2', text: '#2b2522' },
  { value: 'dark', page: '#0b1120', surface: '#111827', line: '#374151', text: '#f3f4f6' },
  { value: 'outline', page: '#ffffff', surface: '#ffffff', line: '#0f172a', text: '#0f172a' },
] as const

type Key = keyof AppearanceValues

function ThemeCard({ theme, selected, label, accent, onSelect }: { theme: (typeof THEMES)[number]; selected: boolean; label: string; accent: string; onSelect: () => void }) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      onClick={onSelect}
      className={cn(
        'grid gap-2 rounded-lg border p-2 text-left text-sm transition-colors hover:border-primary/60 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none',
        selected && 'border-primary ring-1 ring-primary',
      )}
    >
      <span className="grid gap-1.5 rounded-md p-2" style={{ background: theme.page }} aria-hidden="true">
        <span className="grid gap-1 rounded p-2" style={{ background: theme.surface, border: `1px solid ${theme.line}` }}>
          <span className="h-1.5 w-1/2 rounded-full" style={{ background: theme.text }} />
          <span className="h-3 rounded-sm" style={{ border: `1px solid ${theme.line}` }} />
          <span className="h-3 w-1/3 rounded-sm" style={{ background: accent }} />
        </span>
      </span>
      <span className="px-1 font-medium">{label}</span>
    </button>
  )
}

export function AppearanceSection({ formId }: { formId: string }) {
  const t = useModuleTranslation(FORMS_KEY)
  const { setValue, control } = useFormContext<LeadFormValues>()
  const values = useWatch({ control, name: 'appearance' }) as AppearanceValues
  const set = (key: Key, value: string) => setValue(`appearance.${key}`, value, { shouldDirty: true })
  const choice = (key: Key, options: string[]) => [
    { value: '', label: t('appearance.default') },
    ...options.map((option) => ({ value: option, label: t(`appearance.options.${key}.${option}`) })),
  ]
  const accent = values.accent || '#2563eb'

  return (
    <section className="grid gap-4" aria-labelledby={`${formId}-appearance`}>
      <div className="grid gap-1">
        <h3 id={`${formId}-appearance`} className="text-sm font-semibold">
          {t('appearance.title')}
        </h3>
        <p className="text-xs text-muted-foreground">{t('appearance.description')}</p>
      </div>
      <div className="grid gap-2">
        <span className="text-sm font-medium">{t('appearance.theme')}</span>
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-3">
          <button
            type="button"
            aria-pressed={values.theme === ''}
            onClick={() => set('theme', '')}
            className={cn(
              'grid content-center gap-1 rounded-lg border border-dashed p-3 text-left text-sm hover:border-primary/60 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none',
              values.theme === '' && 'border-primary ring-1 ring-primary',
            )}
          >
            <span className="font-medium">{t('appearance.default')}</span>
            <span className="text-xs text-muted-foreground">{t('appearance.defaultTheme')}</span>
          </button>
          {THEMES.map((theme) => (
            <ThemeCard key={theme.value} theme={theme} accent={accent} selected={values.theme === theme.value} label={t(`appearance.options.theme.${theme.value}`)} onSelect={() => set('theme', theme.value)} />
          ))}
        </div>
      </div>
      <Field label={t('appearance.accent')} description={t('appearance.accentHelp')}>
        {() => (
          <div className="flex flex-wrap items-center gap-2">
            <HexColorInput value={values.accent} fallback="#2563eb" placeholder="#2563eb" label={t('appearance.accent')} onChange={(value) => set('accent', value)} />
            {values.accent !== '' && (
              <Button type="button" size="sm" variant="ghost" onClick={() => set('accent', '')}>
                {t('appearance.useOrganizationColor')}
              </Button>
            )}
          </div>
        )}
      </Field>
      <div className="grid gap-4 sm:grid-cols-2">
        <Field label={t('appearance.radius')}>
          {() => <SegmentedControl aria-label={t('appearance.radius')} value={values.radius} onValueChange={(value) => set('radius', value)} options={choice('radius', ['sharp', 'rounded', 'pill'])} />}
        </Field>
        <Field label={t('appearance.width')}>
          {() => <SegmentedControl aria-label={t('appearance.width')} value={values.width} onValueChange={(value) => set('width', value)} options={choice('width', ['narrow', 'medium', 'wide'])} />}
        </Field>
        <Field label={t('appearance.surface')}>
          {() => <SegmentedControl aria-label={t('appearance.surface')} value={values.surface} onValueChange={(value) => set('surface', value)} options={choice('surface', ['card', 'plain'])} />}
        </Field>
        <Field label={t('appearance.button')}>
          {() => <SegmentedControl aria-label={t('appearance.button')} value={values.button} onValueChange={(value) => set('button', value)} options={choice('button', ['solid', 'outline'])} />}
        </Field>
        <Field label={t('appearance.buttonWidth')}>
          {() => <SegmentedControl aria-label={t('appearance.buttonWidth')} value={values.button_width} onValueChange={(value) => set('button_width', value)} options={choice('button_width', ['auto', 'full'])} />}
        </Field>
        <Field label={t('appearance.font')}>
          {(control) => (
            <NativeSelect {...control} className="w-full" value={values.font} onValueChange={(value) => set('font', value)}>
              {choice('font', ['system', 'serif', 'rounded']).map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </NativeSelect>
          )}
        </Field>
        <Field label={t('appearance.showTitle')}>
          {(control) => (
            <NativeSelect {...control} className="w-full" value={values.show_title} onValueChange={(value) => set('show_title', value)}>
              <option value="">{t('appearance.default')}</option>
              <option value="yes">{t('appearance.show')}</option>
              <option value="no">{t('appearance.hide')}</option>
            </NativeSelect>
          )}
        </Field>
        <Field label={t('appearance.showLogo')}>
          {(control) => (
            <NativeSelect {...control} className="w-full" value={values.show_logo} onValueChange={(value) => set('show_logo', value)}>
              <option value="">{t('appearance.default')}</option>
              <option value="yes">{t('appearance.show')}</option>
              <option value="no">{t('appearance.hide')}</option>
            </NativeSelect>
          )}
        </Field>
      </div>
      <Field label={t('appearance.intro')} description={t('appearance.introHelp')}>
        {(control) => <Textarea {...control} rows={2} maxLength={200} placeholder={t('appearance.introPlaceholder')} value={values.intro} onChange={(event) => set('intro', event.target.value)} />}
      </Field>
      <Field label={t('appearance.embedMode')} description={t('appearance.embedModeHelp')}>
        {(control) => (
          <NativeSelect {...control} className="w-full sm:max-w-sm" value={values.embed_mode} onValueChange={(value) => set('embed_mode', value)}>
            {choice('embed_mode', ['theme', 'inherit']).map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </NativeSelect>
        )}
      </Field>
    </section>
  )
}
