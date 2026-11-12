import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { SelectField } from '@/components/ui/select-field'
import { Textarea } from '@/components/ui/textarea'
import { useModuleTranslation } from '@/kernel/modules/i18n'

import { useCollections, useEmbeds } from './api'
import { isSafeLink, type Block, type BookingBlock, type CollectionBlock, type FormBlock } from './blocks'

interface FormProps<T extends Block> {
  block: T
  disabled?: boolean
  onChange: (block: Block) => void
}

function Heading({ block, disabled, onChange }: FormProps<FormBlock | BookingBlock>) {
  const t = useModuleTranslation('site-blocks')
  return (
    <>
      <Field label={t('fields.sectionTitle')}>
        {(control) => <Input {...control} value={block.title} maxLength={160} disabled={disabled} onChange={(event) => onChange({ ...block, title: event.target.value })} />}
      </Field>
      <Field label={t('fields.contactText')}>
        {(control) => <Textarea {...control} rows={2} value={block.text} maxLength={1000} disabled={disabled} onChange={(event) => onChange({ ...block, text: event.target.value })} />}
      </Field>
    </>
  )
}

function EmbedPicker({ kind, label, help, empty, value, disabled, onChange }: { kind: string; label: string; help: string; empty: string; value: string; disabled?: boolean; onChange: (value: string) => void }) {
  const t = useModuleTranslation('site-blocks')
  const embeds = useEmbeds()
  const options = embeds.data?.find((source) => source.kind === kind)?.options ?? []
  const missing = value !== '' && embeds.isSuccess && !options.some((option) => option.key === value)
  return (
    <Field label={label} description={options.length ? help : empty} error={missing ? t('connected.missing') : undefined}>
      {(control) => (
        <SelectField
          {...control}
          value={value}
          disabled={disabled || embeds.isPending}
          emptyLabel={t('connected.choose')}
          options={options.map((option) => ({ value: option.key, label: option.label }))}
          onValueChange={onChange}
        />
      )}
    </Field>
  )
}

export function FormBlockForm({ block, disabled, onChange }: FormProps<FormBlock>) {
  const t = useModuleTranslation('site-blocks')
  return (
    <div className="blocks-form">
      <Heading block={block} disabled={disabled} onChange={onChange} />
      <EmbedPicker kind="form" label={t('connected.form')} help={t('connected.formHelp')} empty={t('connected.formEmpty')} value={block.form} disabled={disabled} onChange={(form) => onChange({ ...block, form })} />
    </div>
  )
}

export function BookingBlockForm({ block, disabled, onChange }: FormProps<BookingBlock>) {
  const t = useModuleTranslation('site-blocks')
  return (
    <div className="blocks-form">
      <Heading block={block} disabled={disabled} onChange={onChange} />
      <EmbedPicker kind="booking" label={t('connected.booking')} help={t('connected.bookingHelp')} empty={t('connected.bookingEmpty')} value={block.booking} disabled={disabled} onChange={(booking) => onChange({ ...block, booking })} />
    </div>
  )
}

export function CollectionBlockForm({ block, disabled, onChange }: FormProps<CollectionBlock>) {
  const t = useModuleTranslation('site-blocks')
  const collections = useCollections()
  const list = collections.data ?? []
  const current = `${block.owner}.${block.entity}`
  const missing = block.owner !== '' && collections.isSuccess && !list.some((item) => item.record_type === current)
  const chosenCollection = list.find((item) => item.record_type === current)
  return (
    <div className="blocks-form">
      <Field label={t('fields.sectionTitle')}>
        {(control) => <Input {...control} value={block.title} maxLength={160} disabled={disabled} onChange={(event) => onChange({ ...block, title: event.target.value })} />}
      </Field>
      <Field label={t('connected.collection')} description={list.length ? t('connected.collectionHelp') : t('connected.collectionEmpty')} error={missing ? t('connected.missing') : undefined}>
        {(control) => (
          <SelectField
            {...control}
            value={block.owner ? current : ''}
            disabled={disabled || collections.isPending}
            emptyLabel={t('connected.choose')}
            options={list.map((item) => ({ value: item.record_type, label: item.label }))}
            onValueChange={(value) => {
              const chosen = list.find((item) => item.record_type === value)
              onChange({ ...block, owner: chosen?.owner ?? '', entity: chosen?.entity ?? '', scope_field: '', scope: '' })
            }}
          />
        )}
      </Field>
      {chosenCollection && chosenCollection.scopes.length > 0 && (
        <Field label={t('connected.scope')} description={t('connected.scopeHelp')}>
          {(control) => (
            <SelectField
              {...control}
              value={block.scope}
              disabled={disabled}
              emptyLabel={t('connected.allScopes')}
              options={chosenCollection.scopes.map((scope) => ({ value: scope.id, label: scope.label }))}
              onValueChange={(scope) => onChange({ ...block, scope, scope_field: scope ? chosenCollection.scope_field ?? '' : '' })}
            />
          )}
        </Field>
      )}
      <Field label={t('connected.order')}>
        {(control) => (
          <SelectField
            {...control}
            value={block.sort}
            disabled={disabled}
            options={[
              { value: 'newest', label: t('connected.newest') },
              { value: 'oldest', label: t('connected.oldest') },
            ]}
            onValueChange={(sort) => onChange({ ...block, sort: sort as CollectionBlock['sort'] })}
          />
        )}
      </Field>
      <Field label={t('connected.limit')}>
        {(control) => (
          <SelectField
            {...control}
            value={block.limit}
            disabled={disabled}
            options={(['3', '6', '9', '12'] as const).map((value) => ({ value, label: value }))}
            onValueChange={(limit) => onChange({ ...block, limit: limit as CollectionBlock['limit'] })}
          />
        )}
      </Field>
      <Field label={t('connected.moreLabel')} description={t('connected.moreHelp')}>
        {(control) => <Input {...control} value={block.more_label} maxLength={60} disabled={disabled} onChange={(event) => onChange({ ...block, more_label: event.target.value })} />}
      </Field>
      <Field label={t('fields.link')} description={t('fields.linkHelp')} error={isSafeLink(block.more_link) ? undefined : t('fields.linkInvalid')}>
        {(control) => <Input {...control} value={block.more_link} spellCheck={false} autoComplete="off" placeholder="/blog" disabled={disabled} onChange={(event) => onChange({ ...block, more_link: event.target.value })} />}
      </Field>
    </div>
  )
}
