import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { NativeSelect } from '@/components/ui/native-select'
import { Textarea } from '@/components/ui/textarea'
import { ToggleField } from '@/components/ui/toggle-field'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { useTranslation } from 'react-i18next'

import { localized, useCollections, usePluginBlocks, useSiteDesigns, type PluginBlockField } from './api'
import type { Block, PluginBlock, QueryBlock } from './blocks'

type Translate = ReturnType<typeof useModuleTranslation>

function Choice<T extends string>({ label, value, options, onChange, disabled }: { label: string; value: T; options: readonly T[]; onChange: (value: T) => void; disabled?: boolean; t?: Translate }) {
  return (
    <Field label={label}>
      {(control) => (
        <NativeSelect {...control} className="w-full" disabled={disabled} value={value} onValueChange={(next) => onChange(next as T)}>
          {options.map((option) => (
            <option key={option} value={option}>
              {option}
            </option>
          ))}
        </NativeSelect>
      )}
    </Field>
  )
}

function Labeled<T extends string>({ label, value, options, onChange, disabled }: { label: string; value: T; options: { value: T; label: string }[]; onChange: (value: T) => void; disabled?: boolean }) {
  return (
    <Field label={label}>
      {(control) => (
        <NativeSelect {...control} className="w-full" disabled={disabled} value={value} onValueChange={(next) => onChange(next as T)}>
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

function Text({ label, value, onChange, disabled, description, list }: { label: string; value: string; onChange: (value: string) => void; disabled?: boolean; description?: string; list?: string }) {
  return (
    <Field label={label} description={description}>
      {(control) => <Input {...control} list={list} value={value} maxLength={200} disabled={disabled} autoComplete="off" spellCheck={false} onChange={(event) => onChange(event.target.value)} />}
    </Field>
  )
}

function FieldNames({ id }: { id: string }) {
  const collections = useCollections()
  const names = [...new Set((collections.data ?? []).flatMap((collection) => collection.fields))].sort()
  return (
    <datalist id={id}>
      {names.map((name) => (
        <option key={name} value={name} />
      ))}
    </datalist>
  )
}

function DesignKeys({ kind, label, value, onChange, disabled, extra }: { kind: 'part' | 'pattern' | 'menu'; label: string; value: string; onChange: (value: string) => void; disabled?: boolean; extra?: { value: string; label: string }[] }) {
  const designs = useSiteDesigns(kind)
  const options = [...(extra ?? []), ...(designs.data ?? []).map((design) => ({ value: design.target.slice(kind.length + 1), label: design.name || design.target.slice(kind.length + 1) }))]
  const known = options.some((option) => option.value === value)
  return (
    <Field label={label}>
      {(control) => (
        <NativeSelect {...control} className="w-full" disabled={disabled} value={value} onValueChange={onChange}>
          {!known && <option value={value}>{value || '—'}</option>}
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

export function QueryFields({ block, onChange, disabled }: { block: QueryBlock; onChange: (block: Block) => void; disabled?: boolean }) {
  const t = useModuleTranslation('site-blocks')
  const collections = useCollections()
  const set = (changes: Partial<QueryBlock>) => onChange({ ...block, ...changes })
  const current = `${block.owner}.${block.entity}`
  return (
    <>
      <FieldNames id={`${block.id}-fields`} />
      <Labeled label={t('query.source')} value={block.source} disabled={disabled} options={[{ value: 'inherit', label: t('query.inherit') }, { value: 'custom', label: t('query.custom') }]} onChange={(source) => set({ source })} />
      {block.source === 'custom' && (
        <Field label={t('query.entity')}>
          {(control) => (
            <NativeSelect {...control} className="w-full" disabled={disabled} value={current} onValueChange={(next) => { const [owner, entity] = next.split('.'); set({ owner, entity }) }}>
              <option value=".">—</option>
              {(collections.data ?? []).map((collection) => (
                <option key={collection.record_type} value={`${collection.owner}.${collection.entity}`}>
                  {collection.label}
                </option>
              ))}
            </NativeSelect>
          )}
        </Field>
      )}
      <Text label={t('query.title')} value={block.title} disabled={disabled} onChange={(title) => set({ title })} />
      <Labeled label={t('query.layout')} value={block.layout} disabled={disabled} options={(['grid', 'list', 'featured', 'masonry', 'carousel'] as const).map((value) => ({ value, label: t(`query.layout_${value}`) }))} onChange={(layout) => set({ layout })} />
      <Choice label={t('query.columns')} value={block.columns} disabled={disabled} options={['1', '2', '3', '4'] as const} onChange={(columns) => set({ columns })} />
      <Choice label={block.source === 'inherit' ? t('query.perPage') : t('query.limit')} value={block.limit} disabled={disabled} options={['3', '4', '6', '9', '12', '24'] as const} onChange={(limit) => set({ limit })} />
      {block.source === 'custom' && (
        <>
          <Labeled label={t('query.sort')} value={block.sort} disabled={disabled} options={[{ value: 'newest', label: t('query.newest') }, { value: 'oldest', label: t('query.oldest') }]} onChange={(sort) => set({ sort })} />
          <Text label={t('query.orderBy')} description={t('query.orderByHelp')} list={`${block.id}-fields`} value={block.order_by} disabled={disabled} onChange={(order_by) => set({ order_by })} />
          <Text label={t('query.scopeField')} description={t('query.scopeFieldHelp')} list={`${block.id}-fields`} value={block.scope_field} disabled={disabled} onChange={(scope_field) => set({ scope_field })} />
          <Text label={t('query.matchField')} description={t('query.matchFieldHelp')} list={`${block.id}-fields`} value={block.match_field} disabled={disabled} onChange={(match_field) => set({ match_field })} />
          <ToggleField label={t('query.excludeCurrent')} checked={block.exclude_current} disabled={disabled} onCheckedChange={(exclude_current) => set({ exclude_current })} />
        </>
      )}
      <Text label={t('query.emptyText')} value={block.empty_text} disabled={disabled} onChange={(empty_text) => set({ empty_text })} />
      <p className="blocks-muted">{t('query.childrenHelp')}</p>
    </>
  )
}

export function SiteFields({ block, onChange, disabled }: { block: Block; onChange: (block: Block) => void; disabled?: boolean }) {
  const t = useModuleTranslation('site-blocks')
  const fieldsId = `${block.id}-fields`
  switch (block.type) {
    case 'site_logo':
      return (
        <div className="blocks-form">
          <Labeled label={t('site.show')} value={block.show} disabled={disabled} options={(['logo_and_title', 'logo', 'title'] as const).map((value) => ({ value, label: t(`site.show_${value}`) }))} onChange={(show) => onChange({ ...block, show })} />
          <ToggleField label={t('site.tagline')} checked={block.tagline} disabled={disabled} onCheckedChange={(tagline) => onChange({ ...block, tagline })} />
        </div>
      )
    case 'navigation':
      return (
        <div className="blocks-form">
          <DesignKeys kind="menu" label={t('site.menu')} value={block.menu} disabled={disabled} extra={[{ value: 'main', label: t('site.menu_main') }, { value: 'footer', label: t('site.menu_footer') }]} onChange={(menu) => onChange({ ...block, menu })} />
          <Labeled label={t('site.layout')} value={block.layout} disabled={disabled} options={(['horizontal', 'vertical'] as const).map((value) => ({ value, label: t(`site.layout_${value}`) }))} onChange={(layout) => onChange({ ...block, layout })} />
          <Labeled label={t('site.mobile')} value={block.mobile} disabled={disabled} options={(['panel', 'list'] as const).map((value) => ({ value, label: t(`site.mobile_${value}`) }))} onChange={(mobile) => onChange({ ...block, mobile })} />
        </div>
      )
    case 'search':
      return (
        <div className="blocks-form">
          <Labeled label={t('site.display')} value={block.display} disabled={disabled} options={(['icon', 'field'] as const).map((value) => ({ value, label: t(`site.display_${value}`) }))} onChange={(display) => onChange({ ...block, display })} />
        </div>
      )
    case 'template_part':
      return (
        <div className="blocks-form">
          <DesignKeys kind="part" label={t('site.part')} value={block.part} disabled={disabled} extra={[{ value: 'header', label: t('site.part_header') }, { value: 'footer', label: t('site.part_footer') }]} onChange={(part) => onChange({ ...block, part })} />
        </div>
      )
    case 'pattern':
      return (
        <div className="blocks-form">
          <DesignKeys kind="pattern" label={t('site.pattern')} value={block.pattern} disabled={disabled} onChange={(pattern) => onChange({ ...block, pattern })} />
        </div>
      )
    case 'record_title':
      return (
        <div className="blocks-form">
          <Choice label={t('record.level')} value={block.level} disabled={disabled} options={['h1', 'h2', 'h3', 'p'] as const} onChange={(level) => onChange({ ...block, level })} />
          <ToggleField label={t('record.link')} checked={block.link} disabled={disabled} onCheckedChange={(link) => onChange({ ...block, link })} />
        </div>
      )
    case 'record_field':
      return (
        <div className="blocks-form">
          <FieldNames id={fieldsId} />
          <Text label={t('record.field')} list={fieldsId} value={block.field} disabled={disabled} onChange={(field) => onChange({ ...block, field })} />
          <Labeled label={t('record.format')} value={block.format} disabled={disabled} options={(['text', 'date', 'datetime', 'markdown', 'number', 'reading_time'] as const).map((value) => ({ value, label: t(`record.format_${value}`) }))} onChange={(format) => onChange({ ...block, format })} />
          <Text label={t('record.label')} value={block.label} disabled={disabled} onChange={(label) => onChange({ ...block, label })} />
        </div>
      )
    case 'record_image':
      return (
        <div className="blocks-form">
          <FieldNames id={fieldsId} />
          <Text label={t('record.field')} list={fieldsId} value={block.field} disabled={disabled} onChange={(field) => onChange({ ...block, field })} />
          <Labeled label={t('record.ratio')} value={block.ratio} disabled={disabled} options={(['original', 'landscape', 'square', 'portrait', 'wide'] as const).map((value) => ({ value, label: t(`record.ratio_${value}`) }))} onChange={(ratio) => onChange({ ...block, ratio })} />
          <ToggleField label={t('record.link')} checked={block.link} disabled={disabled} onCheckedChange={(link) => onChange({ ...block, link })} />
          <ToggleField label={t('record.caption')} checked={block.caption} disabled={disabled} onCheckedChange={(caption) => onChange({ ...block, caption })} />
        </div>
      )
    case 'record_terms':
      return (
        <div className="blocks-form">
          <Labeled label={t('record.vocabulary')} value={block.vocabulary} disabled={disabled} options={(['all', 'categories', 'tags'] as const).map((value) => ({ value, label: t(`record.vocabulary_${value}`) }))} onChange={(vocabulary) => onChange({ ...block, vocabulary })} />
          <Labeled label={t('record.display')} value={block.display} disabled={disabled} options={(['chips', 'plain', 'kicker'] as const).map((value) => ({ value, label: t(`record.display_${value}`) }))} onChange={(display) => onChange({ ...block, display })} />
        </div>
      )
    case 'record_body':
      return (
        <div className="blocks-form">
          <FieldNames id={fieldsId} />
          <Text label={t('record.field')} list={fieldsId} value={block.field} disabled={disabled} onChange={(field) => onChange({ ...block, field })} />
        </div>
      )
    case 'record_link':
      return (
        <div className="blocks-form">
          <Text label={t('record.linkLabel')} value={block.label} disabled={disabled} onChange={(label) => onChange({ ...block, label })} />
        </div>
      )
    case 'archive_title':
      return (
        <div className="blocks-form">
          <Choice label={t('record.level')} value={block.level} disabled={disabled} options={['h1', 'h2'] as const} onChange={(level) => onChange({ ...block, level })} />
        </div>
      )
    case 'query_pagination':
      return (
        <div className="blocks-form">
          <Labeled label={t('site.display')} value={block.display} disabled={disabled} options={(['arrows', 'numbers'] as const).map((value) => ({ value, label: t(`record.pagination_${value}`) }))} onChange={(display) => onChange({ ...block, display })} />
        </div>
      )
    default:
      return <p className="blocks-muted">{t('site.noSettings')}</p>
  }
}

function PluginField({ field, value, onChange, disabled, language }: { field: PluginBlockField; value: unknown; onChange: (value: unknown) => void; disabled?: boolean; language: string }) {
  const label = localized(field.label, language) || field.key
  const description = localized(field.description, language) || undefined
  const current = value === undefined ? field.default : value
  switch (field.kind) {
    case 'flag':
      return <ToggleField label={label} description={description} checked={Boolean(current)} disabled={disabled} onCheckedChange={onChange} />
    case 'select':
      return (
        <Field label={label} description={description}>
          {(control) => (
            <NativeSelect {...control} className="w-full" disabled={disabled} value={String(current ?? '')} onValueChange={onChange}>
              {(field.options ?? []).map((option) => (
                <option key={option.value} value={option.value}>
                  {localized(option.label, language) || option.value}
                </option>
              ))}
            </NativeSelect>
          )}
        </Field>
      )
    case 'number':
      return (
        <Field label={label} description={description}>
          {(control) => <Input {...control} type="number" disabled={disabled} value={current === undefined || current === null ? '' : String(current)} onChange={(event) => onChange(event.target.value === '' ? null : Number(event.target.value))} />}
        </Field>
      )
    case 'long_text':
      return (
        <Field label={label} description={description}>
          {(control) => <Textarea {...control} rows={4} maxLength={2000} disabled={disabled} value={String(current ?? '')} onChange={(event) => onChange(event.target.value)} />}
        </Field>
      )
    default:
      return (
        <Field label={label} description={description}>
          {(control) => <Input {...control} maxLength={2000} disabled={disabled} value={String(current ?? '')} onChange={(event) => onChange(event.target.value)} />}
        </Field>
      )
  }
}

export function PluginFields({ block, onChange, disabled }: { block: PluginBlock; onChange: (block: Block) => void; disabled?: boolean }) {
  const t = useModuleTranslation('site-blocks')
  const { i18n } = useTranslation()
  const catalog = usePluginBlocks()
  const definition = catalog.data?.find((candidate) => candidate.type === block.type)
  if (!definition) return <p className="blocks-muted">{catalog.isPending ? '…' : t('site.unknownPlugin')}</p>
  return (
    <div className="blocks-form">
      {localized(definition.description, i18n.language) && <p className="blocks-muted">{localized(definition.description, i18n.language)}</p>}
      {definition.fields.length === 0 && <p className="blocks-muted">{t('site.noSettings')}</p>}
      {definition.fields.map((field) => (
        <PluginField key={field.key} field={field} value={block[field.key]} disabled={disabled} language={i18n.language} onChange={(value) => onChange({ ...block, [field.key]: value } as Block)} />
      ))}
    </div>
  )
}
