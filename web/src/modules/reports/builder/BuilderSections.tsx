import { ArrowDown, ArrowUp, Plus, Trash2 } from 'lucide-react'
import { useId, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { FilterBuilder, type DraftGroup, type FilterFieldExtension } from '@/features/records/filters'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import type { FieldSchema } from '@/features/records/types'
import { cn } from '@/lib/utils'
import {
  TIME_GRANULARITIES,
  type AggregateField,
  type AggregateFunction,
  type AggregateMetric,
  type TimeGranularity,
} from '@/features/records/aggregates'
import type { PeriodPreset, ReportVisualization } from './api'
import { metricName, type GroupOption } from './options'
import {
  MAX_GROUPS,
  MAX_LIMIT,
  MAX_METRICS,
  PERIOD_PRESETS,
  VISUALIZATIONS,
  isDurationMetric,
  isMetricComplete,
  metricKeyOf,
  needsField,
  withGranularity,
  type ReportDraft,
} from './model'
import { Field } from '@/components/ui/field'

const FUNCTIONS: AggregateFunction[] = ['count', 'sum', 'avg', 'min', 'max']
const DURATION_UNITS = ['days', 'hours'] as const
type DurationUnit = (typeof DURATION_UNITS)[number]


export function Section({ title, description, children }: { title: string; description?: string; children: ReactNode }) {
  return (
    <Card>
      <CardHeader className="gap-1">
        <CardTitle className="text-base">{title}</CardTitle>
        {description && <CardDescription>{description}</CardDescription>}
      </CardHeader>
      <CardContent className="grid gap-3">{children}</CardContent>
    </Card>
  )
}

interface DraftProps {
  draft: ReportDraft
  onChange: (update: Partial<ReportDraft>) => void
}

export function GroupsSection({ draft, onChange, options }: DraftProps & { options: GroupOption[] }) {
  const { t } = useTranslation('reports', { keyPrefix: 'builder' })
  const { t: tReports } = useTranslation('reports')
  const groups = draft.groupBy
  const optionFor = (key: string) => options.find((option) => option.key === key)
  const update = (index: number, field: string) => {
    const option = optionFor(field)
    onChange({
      groupBy: groups.map((group, position) =>
        position === index ? withGranularity({ field }, option?.temporal ?? false) : group,
      ),
    })
  }
  return (
    <Section title={t('sections.groups.title')} description={t('sections.groups.description')}>
      {groups.length === 0 && <p className="text-sm text-muted-foreground">{t('sections.groups.empty')}</p>}
      {groups.map((group, index) => {
        const option = optionFor(group.field)
        return (
          <div key={index} className="flex flex-wrap items-center gap-2">
            <NativeSelect
              className="min-w-0 flex-1 basis-48"
              aria-label={t('sections.groups.field', { position: index + 1 })}
              value={group.field}
              onChange={(event) => update(index, event.target.value)}
            >
              <option value="">{t('common.choose')}</option>
              {options.map((candidate) => (
                <option key={candidate.key} value={candidate.key}>
                  {candidate.label}
                </option>
              ))}
            </NativeSelect>
            {option?.temporal && (
              <NativeSelect
                className="w-36"
                aria-label={t('sections.groups.granularity')}
                value={group.granularity ?? 'day'}
                onChange={(event) =>
                  onChange({
                    groupBy: groups.map((item, position) =>
                      position === index ? { ...item, granularity: event.target.value as TimeGranularity } : item,
                    ),
                  })
                }
              >
                {TIME_GRANULARITIES.map((granularity) => (
                  <option key={granularity} value={granularity}>
                    {tReports(`granularity.${granularity}`)}
                  </option>
                ))}
              </NativeSelect>
            )}
            <Button
              type="button"
              variant="ghost"
              size="icon"
              aria-label={t('common.remove')}
              onClick={() => onChange({ groupBy: groups.filter((_, position) => position !== index) })}
            >
              <Trash2 />
            </Button>
          </div>
        )
      })}
      <div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={groups.length >= MAX_GROUPS}
          onClick={() => onChange({ groupBy: [...groups, { field: '' }] })}
        >
          <Plus />
          {t('sections.groups.add')}
        </Button>
      </div>
    </Section>
  )
}

export function MetricsSection({ draft, onChange, fields }: DraftProps & { fields: AggregateField[] }) {
  const { t } = useTranslation('reports', { keyPrefix: 'builder' })
  const { t: tReports } = useTranslation('reports')
  const metrics = draft.metrics
  const fieldsFor = (fn: AggregateFunction) =>
    fields.filter((field) => !field.is_virtual && field.functions.includes(fn))
  const dateFields = fields.filter((field) => !field.is_virtual && field.temporal)
  const replace = (index: number, next: AggregateMetric) => {
    onChange({ metrics: metrics.map((metric, position) => (position === index ? next : metric)) })
  }
  const update = (index: number, fn: AggregateFunction, field: string | null) => {
    replace(index, field ? { fn, field } : { fn })
  }
  const changeFunction = (index: number, metric: AggregateMetric, fn: AggregateFunction) => {
    if (isDurationMetric(metric) && fn !== 'count') {
      replace(index, { ...metric, fn })
      return
    }
    const keep = !isDurationMetric(metric) && metric.field && fieldsFor(fn).some((field) => field.key === metric.field)
    update(index, fn, keep ? (metric.field ?? null) : null)
  }
  const changeMeasure = (index: number, metric: AggregateMetric, duration: boolean) => {
    replace(index, duration ? { fn: metric.fn, field: null, to: '', unit: 'days' } : { fn: metric.fn })
  }
  return (
    <Section title={t('sections.metrics.title')} description={t('sections.metrics.description')}>
      {metrics.map((metric, index) => {
        const candidates = fieldsFor(metric.fn)
        const duration = isDurationMetric(metric)
        return (
          <div key={index} className="flex flex-wrap items-center gap-2">
            <NativeSelect
              className="w-36"
              aria-label={t('sections.metrics.function', { position: index + 1 })}
              value={metric.fn}
              onChange={(event) => changeFunction(index, metric, event.target.value as AggregateFunction)}
            >
              {FUNCTIONS.map((fn) => (
                <option key={fn} value={fn}>
                  {tReports(`functions.${fn}`)}
                </option>
              ))}
            </NativeSelect>
            {needsField(metric.fn) && (
              <NativeSelect
                className="w-48"
                aria-label={t('sections.metrics.measure', { position: index + 1 })}
                value={duration ? 'duration' : 'value'}
                onChange={(event) => changeMeasure(index, metric, event.target.value === 'duration')}
              >
                <option value="value">{t('sections.metrics.measureValue')}</option>
                <option value="duration" disabled={dateFields.length < 2}>
                  {t('sections.metrics.measureDuration')}
                </option>
              </NativeSelect>
            )}
            {duration ? (
              <>
                <NativeSelect
                  className="min-w-0 flex-1 basis-36"
                  aria-label={t('sections.metrics.from', { position: index + 1 })}
                  value={metric.field ?? ''}
                  aria-invalid={!metric.field}
                  onChange={(event) => replace(index, { ...metric, field: event.target.value || null })}
                >
                  <option value="">{t('sections.metrics.fromPlaceholder')}</option>
                  {dateFields.map((field) => (
                    <option key={field.key} value={field.key}>
                      {field.label}
                    </option>
                  ))}
                </NativeSelect>
                <NativeSelect
                  className="min-w-0 flex-1 basis-36"
                  aria-label={t('sections.metrics.to', { position: index + 1 })}
                  value={metric.to ?? ''}
                  aria-invalid={!metric.to || metric.to === metric.field}
                  onChange={(event) => replace(index, { ...metric, to: event.target.value })}
                >
                  <option value="">{t('sections.metrics.toPlaceholder')}</option>
                  {dateFields
                    .filter((field) => field.key !== metric.field)
                    .map((field) => (
                      <option key={field.key} value={field.key}>
                        {field.label}
                      </option>
                    ))}
                </NativeSelect>
                <NativeSelect
                  className="w-28"
                  aria-label={t('sections.metrics.unit', { position: index + 1 })}
                  value={metric.unit ?? 'days'}
                  onChange={(event) => replace(index, { ...metric, unit: event.target.value as DurationUnit })}
                >
                  {DURATION_UNITS.map((unit) => (
                    <option key={unit} value={unit}>
                      {tReports(`durationUnits.${unit}`)}
                    </option>
                  ))}
                </NativeSelect>
              </>
            ) : (
              <NativeSelect
                className="min-w-0 flex-1 basis-40"
                aria-label={t('sections.metrics.field', { position: index + 1 })}
                value={metric.field ?? ''}
                aria-invalid={needsField(metric.fn) && !metric.field}
                onChange={(event) => update(index, metric.fn, event.target.value || null)}
              >
                <option value="">{needsField(metric.fn) ? t('common.choose') : t('sections.metrics.records')}</option>
                {candidates.map((field) => (
                  <option key={field.key} value={field.key}>
                    {field.label}
                  </option>
                ))}
              </NativeSelect>
            )}
            <Button
              type="button"
              variant="ghost"
              size="icon"
              aria-label={t('common.remove')}
              disabled={metrics.length <= 1}
              onClick={() => onChange({ metrics: metrics.filter((_, position) => position !== index) })}
            >
              <Trash2 />
            </Button>
          </div>
        )
      })}
      <div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={metrics.length >= MAX_METRICS}
          onClick={() => onChange({ metrics: [...metrics, { fn: 'count' }] })}
        >
          <Plus />
          {t('sections.metrics.add')}
        </Button>
      </div>
    </Section>
  )
}

export function FiltersSection({
  schema,
  value,
  onChange,
  extensions,
}: {
  schema: FieldSchema | undefined
  value: DraftGroup
  onChange: (value: DraftGroup) => void
  extensions?: FilterFieldExtension[]
}) {
  const { t } = useTranslation('reports', { keyPrefix: 'builder' })
  return (
    <Section title={t('sections.filters.title')} description={t('sections.filters.description')}>
      {schema ? (
        <FilterBuilder schema={schema} value={value} onChange={onChange} extensions={extensions} allowRelated />
      ) : (
        <p className="text-sm text-muted-foreground">{t('common.loading')}</p>
      )}
    </Section>
  )
}

export function PeriodSection({ draft, onChange, fields }: DraftProps & { fields: AggregateField[] }) {
  const { t } = useTranslation('reports', { keyPrefix: 'builder' })
  const ids = { enabled: useId(), field: useId(), preset: useId(), from: useId(), to: useId(), compare: useId() }
  const temporal = fields.filter((field) => field.temporal && !field.is_virtual)
  const period = draft.period
  return (
    <Section title={t('sections.period.title')} description={t('sections.period.description')}>
      <div className="flex items-center gap-2">
        <Checkbox
          id={ids.enabled}
          checked={period !== null}
          disabled={temporal.length === 0}
          onCheckedChange={(checked) =>
            onChange({
              period: checked === true ? { field: temporal[0]?.key ?? '', preset: 'month', from: '', to: '' } : null,
              compare: checked === true ? draft.compare : false,
            })
          }
        />
        <Label htmlFor={ids.enabled}>{t('sections.period.enabled')}</Label>
      </div>
      {period && (
        <div className="grid gap-3 sm:grid-cols-2">
          <div className="grid gap-2.5">
            <Label htmlFor={ids.field}>{t('sections.period.field')}</Label>
            <NativeSelect
              id={ids.field}
              value={period.field}
              onChange={(event) => onChange({ period: { ...period, field: event.target.value } })}
            >
              {temporal.map((field) => (
                <option key={field.key} value={field.key}>
                  {field.label}
                </option>
              ))}
            </NativeSelect>
          </div>
          <div className="grid gap-2.5">
            <Label htmlFor={ids.preset}>{t('sections.period.preset')}</Label>
            <NativeSelect
              id={ids.preset}
              value={period.preset}
              onChange={(event) => onChange({ period: { ...period, preset: event.target.value as PeriodPreset } })}
            >
              {PERIOD_PRESETS.map((preset) => (
                <option key={preset} value={preset}>
                  {t(`presets.${preset}`)}
                </option>
              ))}
            </NativeSelect>
          </div>
          {period.preset === 'custom' && (
            <>
              <Field label={t('sections.period.from')}>
                {(control) => <Input
                  {...control}
                  type="date"
                  value={period.from}
                  onChange={(event) => onChange({ period: { ...period, from: event.target.value } })}
                />}
              </Field>
              <Field label={t('sections.period.to')}>
                {(control) => <Input
                  {...control}
                  type="date"
                  value={period.to}
                  aria-invalid={Boolean(period.from && period.to && period.from > period.to)}
                  onChange={(event) => onChange({ period: { ...period, to: event.target.value } })}
                />}
              </Field>
            </>
          )}
          <div className="flex items-center gap-2 sm:col-span-2">
            <Checkbox
              id={ids.compare}
              checked={draft.compare}
              onCheckedChange={(checked) => onChange({ compare: checked === true })}
            />
            <Label htmlFor={ids.compare}>{t('sections.period.compare')}</Label>
          </div>
        </div>
      )}
      {temporal.length === 0 && <p className="text-sm text-muted-foreground">{t('sections.period.noFields')}</p>}
    </Section>
  )
}

export function SortSection({
  draft,
  onChange,
  options,
  fields,
}: DraftProps & { options: GroupOption[]; fields: AggregateField[] }) {
  const { t } = useTranslation('reports', { keyPrefix: 'builder' })
  const { t: tReports } = useTranslation('reports')
  const keys = [
    ...draft.groupBy
      .filter((group) => group.field)
      .map((group) => ({
        key: group.field,
        label: options.find((option) => option.key === group.field)?.label ?? group.field,
      })),
    ...draft.metrics
      .filter(isMetricComplete)
      .map((metric) => ({
        key: metricKeyOf(metric),
        label: metricName(metric, fields, tReports),
      })),
  ]
  const sort = draft.sort
  return (
    <Section title={t('sections.sort.title')} description={t('sections.sort.description')}>
      {sort.map((item, index) => (
        <div key={index} className="flex flex-wrap items-center gap-2">
          <NativeSelect
            className="min-w-0 flex-1 basis-48"
            aria-label={t('sections.sort.key', { position: index + 1 })}
            value={item.key}
            onChange={(event) =>
              onChange({
                sort: sort.map((entry, position) => (position === index ? { ...entry, key: event.target.value } : entry)),
              })
            }
          >
            <option value="">{t('common.choose')}</option>
            {keys.map((key) => (
              <option key={key.key} value={key.key}>
                {key.label}
              </option>
            ))}
          </NativeSelect>
          <Button
            type="button"
            variant="outline"
            size="sm"
            aria-label={item.direction === 'desc' ? t('sections.sort.desc') : t('sections.sort.asc')}
            onClick={() =>
              onChange({
                sort: sort.map((entry, position) =>
                  position === index ? { ...entry, direction: entry.direction === 'desc' ? 'asc' : 'desc' } : entry,
                ),
              })
            }
          >
            {item.direction === 'desc' ? <ArrowDown /> : <ArrowUp />}
            {item.direction === 'desc' ? t('sections.sort.desc') : t('sections.sort.asc')}
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label={t('common.remove')}
            onClick={() => onChange({ sort: sort.filter((_, position) => position !== index) })}
          >
            <Trash2 />
          </Button>
        </div>
      ))}
      <div className="flex flex-wrap items-end justify-between gap-3">
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={sort.length >= keys.length}
          onClick={() => onChange({ sort: [...sort, { key: '', direction: 'desc' }] })}
        >
          <Plus />
          {t('sections.sort.add')}
        </Button>
        <Field label={t('sections.sort.limit')}>
          {(control) => <Input placeholder={t('placeholders.limit')}
            {...control}
            type="number"
            min={1}
            max={MAX_LIMIT}
            className="w-28"
            value={draft.limit}
            onChange={(event) => {
              const value = Math.round(Number(event.target.value))
              onChange({ limit: Number.isFinite(value) ? Math.min(MAX_LIMIT, Math.max(1, value)) : 1 })
            }}
          />}
        </Field>
      </div>
    </Section>
  )
}

export function VisualizationSection({ draft, onChange, fields }: DraftProps & { fields: AggregateField[] }) {
  const { t } = useTranslation('reports', { keyPrefix: 'builder' })
  const { t: tReports } = useTranslation('reports')
  const metricId = useId()
  const metrics = draft.metrics.filter(isMetricComplete)
  return (
    <Section title={t('sections.visualization.title')}>
      <div role="radiogroup" aria-label={t('sections.visualization.title')} className="flex flex-wrap gap-2">
        {VISUALIZATIONS.map((visualization: ReportVisualization) => (
          <button
            key={visualization}
            type="button"
            role="radio"
            aria-checked={draft.visualization === visualization}
            onClick={() => onChange({ visualization })}
            className={cn(
              'rounded-md border px-3 py-1.5 text-sm transition-colors',
              draft.visualization === visualization
                ? 'border-primary-solid bg-primary-solid text-primary-foreground'
                : 'hover:bg-accent hover:text-accent-foreground',
            )}
          >
            {t(`visualizations.${visualization}`)}
          </button>
        ))}
      </div>
      {draft.visualization !== 'table' && metrics.length > 1 && (
        <div className="grid gap-2.5">
          <Label htmlFor={metricId}>{t('sections.visualization.metric')}</Label>
          <NativeSelect
            id={metricId}
            value={draft.chartMetric ?? metricKeyOf(metrics[0])}
            onChange={(event) => onChange({ chartMetric: event.target.value })}
          >
            {metrics.map((metric) => (
              <option key={metricKeyOf(metric)} value={metricKeyOf(metric)}>
                {metricName(metric, fields, tReports)}
              </option>
            ))}
          </NativeSelect>
        </div>
      )}
    </Section>
  )
}
