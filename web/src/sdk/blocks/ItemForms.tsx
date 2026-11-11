import { errorMessage } from '@/components/feedback'
import { PhoneFieldInput } from '@/components/fields/LazyPhoneFieldInput'
import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { SegmentedControl } from '@/components/ui/segmented-control'
import { Textarea } from '@/components/ui/textarea'
import { toast } from '@/components/ui/toast-store'
import { ToggleField } from '@/components/ui/toggle-field'
import { uploadFile } from '@/features/files'
import { useRecordFormatting } from '@/features/records/formatting'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { useQueryClient } from '@tanstack/react-query'
import { ArrowDown, ArrowUp, ImagePlus, Plus, Trash2 } from 'lucide-react'
import { useRef, useState, type ReactNode } from 'react'

import { ITEM_ICONS } from './blockIcons'
import {
  ICONS,
  IMAGE_TYPES,
  ITEM_LIMITS,
  isSafeLink,
  isValidEmail,
  newColumn,
  newFaqItem,
  newGalleryItem,
  newTestimonial,
  videoOf,
  embedOf,
  type Block,
  type ColumnsBlock,
  type ContactBlock,
  type FaqBlock,
  type GalleryBlock,
  type IconName,
  type TestimonialsBlock,
  type VideoBlock,
  type EmbedBlock,
} from './blocks'
import { ImageField } from './ImageField'
import type { BlockRecord } from './api'

type Translate = ReturnType<typeof useModuleTranslation>

interface ItemListProps<T> {
  items: T[]
  limit: number
  disabled?: boolean
  itemLabel: (index: number) => string
  addLabel: string
  onChange: (items: T[]) => void
  create: () => T
  render: (item: T, update: (changes: Partial<T>) => void) => ReactNode
}

function ItemList<T>({ items, limit, disabled, itemLabel, addLabel, onChange, create, render }: ItemListProps<T>) {
  const t = useModuleTranslation('site-blocks')
  const move = (index: number, offset: number) => {
    const next = [...items]
    const [item] = next.splice(index, 1)
    next.splice(index + offset, 0, item)
    onChange(next)
  }
  return (
    <div className="blocks-items">
      {items.map((item, index) => (
        <fieldset key={index} className="blocks-fieldset">
          <legend>{itemLabel(index)}</legend>
          <div className="blocks-item-tools">
            <Button type="button" variant="ghost" size="icon" aria-label={t('items.moveUp')} title={t('items.moveUp')} disabled={disabled || index === 0} onClick={() => move(index, -1)}>
              <ArrowUp aria-hidden />
            </Button>
            <Button type="button" variant="ghost" size="icon" aria-label={t('items.moveDown')} title={t('items.moveDown')} disabled={disabled || index === items.length - 1} onClick={() => move(index, 1)}>
              <ArrowDown aria-hidden />
            </Button>
            <Button type="button" variant="ghost" size="icon" aria-label={t('items.remove')} title={t('items.remove')} disabled={disabled} onClick={() => onChange(items.filter((_, position) => position !== index))}>
              <Trash2 aria-hidden />
            </Button>
          </div>
          {render(item, (changes) => onChange(items.map((current, position) => (position === index ? { ...current, ...changes } : current))))}
        </fieldset>
      ))}
      {items.length < limit ? (
        <Button type="button" variant="outline" size="sm" disabled={disabled} onClick={() => onChange([...items, create()])}>
          <Plus aria-hidden />
          {addLabel}
        </Button>
      ) : (
        <p className="blocks-muted">{t('items.limit', { count: limit })}</p>
      )}
    </div>
  )
}

function TextInput({ label, value, maxLength, disabled, description, onChange }: { label: string; value: string; maxLength: number; disabled?: boolean; description?: string; onChange: (value: string) => void }) {
  return (
    <Field label={label} description={description}>
      {(control) => <Input {...control} value={value} maxLength={maxLength} disabled={disabled} onChange={(event) => onChange(event.target.value)} />}
    </Field>
  )
}

function TextAreaInput({ label, value, maxLength, rows = 4, disabled, description, onChange }: { label: string; value: string; maxLength: number; rows?: number; disabled?: boolean; description?: string; onChange: (value: string) => void }) {
  return (
    <Field label={label} description={description}>
      {(control) => <Textarea {...control} value={value} rows={rows} maxLength={maxLength} disabled={disabled} onChange={(event) => onChange(event.target.value)} />}
    </Field>
  )
}

function GalleryUpload({ record, disabled, room, onUploaded, t }: { record: BlockRecord; disabled?: boolean; room: number; onUploaded: (ids: string[]) => void; t: Translate }) {
  const client = useQueryClient()
  const input = useRef<HTMLInputElement>(null)
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null)

  async function upload(files: File[]) {
    const images = files.filter((file) => IMAGE_TYPES.includes(file.type)).slice(0, room)
    if (images.length < files.length) toast({ title: t('images.someSkipped') })
    const ids: string[] = []
    setProgress({ done: 0, total: images.length })
    try {
      for (const file of images) {
        const uploaded = await uploadFile({ file, target: record })
        ids.push(uploaded.file.id)
        setProgress({ done: ids.length, total: images.length })
      }
    } catch (failure) {
      toast({ title: errorMessage(failure), variant: 'destructive' })
    } finally {
      setProgress(null)
      if (input.current) input.current.value = ''
      if (ids.length) {
        onUploaded(ids)
        await client.invalidateQueries({ queryKey: ['files'] })
      }
    }
  }

  return (
    <>
      <input
        ref={input}
        type="file"
        multiple
        accept={IMAGE_TYPES.join(',')}
        className="blocks-hidden"
        tabIndex={-1}
        aria-hidden
        onChange={(event) => void upload(Array.from(event.target.files ?? []))}
      />
      <Button type="button" variant="outline" disabled={disabled || room <= 0 || progress !== null} onClick={() => input.current?.click()}>
        <ImagePlus aria-hidden />
        {progress ? t('images.uploadingMany', { done: progress.done, total: progress.total }) : t('images.addMany')}
      </Button>
    </>
  )
}

function IconPicker({ value, disabled, onChange, t }: { value: IconName; disabled?: boolean; onChange: (icon: IconName) => void; t: Translate }) {
  return (
    <Field label={t('fields.icon')}>
      {() => (
        <div className="blocks-icons" role="radiogroup" aria-label={t('fields.icon')}>
          {ICONS.map((icon) => {
            const Icon = ITEM_ICONS[icon]
            return (
              <button
                key={icon}
                type="button"
                role="radio"
                aria-checked={value === icon}
                aria-label={t(`icons.${icon}`)}
                title={t(`icons.${icon}`)}
                className="blocks-icon-option"
                disabled={disabled}
                onClick={() => onChange(icon)}
              >
                <Icon aria-hidden size={18} />
              </button>
            )
          })}
        </div>
      )}
    </Field>
  )
}

interface FormProps<T extends Block> {
  block: T
  record: BlockRecord
  disabled?: boolean
  onChange: (block: Block) => void
}

export function GalleryForm({ block, record, disabled, onChange }: FormProps<GalleryBlock>) {
  const t = useModuleTranslation('site-blocks')
  const room = ITEM_LIMITS.gallery - block.items.length
  return (
    <div className="blocks-form">
      <TextInput label={t('fields.sectionTitle')} value={block.title} maxLength={160} disabled={disabled} onChange={(title) => onChange({ ...block, title })} />
      <Field label={t('fields.layout')}>
        {() => (
          <SegmentedControl<GalleryBlock['layout']>
            aria-label={t('fields.layout')}
            value={block.layout}
            onValueChange={(layout) => onChange({ ...block, layout })}
            options={[
              { value: 'grid', label: t('fields.layoutGrid'), disabled },
              { value: 'carousel', label: t('fields.layoutCarousel'), disabled },
            ]}
          />
        )}
      </Field>
      {block.layout === 'grid' && (
        <Field label={t('fields.perRow')}>
          {() => (
            <SegmentedControl<GalleryBlock['columns']>
              aria-label={t('fields.perRow')}
              value={block.columns}
              onValueChange={(columns) => onChange({ ...block, columns })}
              options={(['2', '3', '4'] as const).map((value) => ({ value, label: value, disabled }))}
            />
          )}
        </Field>
      )}
      <GalleryUpload record={record} disabled={disabled} room={room} t={t} onUploaded={(ids) => onChange({ ...block, items: [...block.items, ...ids.map((id) => newGalleryItem(id))] })} />
      <ItemList
        items={block.items}
        limit={ITEM_LIMITS.gallery}
        disabled={disabled}
        itemLabel={(index) => t('items.photo', { number: index + 1 })}
        addLabel={t('items.addPhoto')}
        create={() => newGalleryItem()}
        onChange={(items) => onChange({ ...block, items })}
        render={(item, update) => (
          <>
            <ImageField record={record} value={item.image} disabled={disabled} onChange={(image) => update({ image })} />
            <TextInput label={t('fields.alt')} description={t('fields.altHelp')} value={item.alt} maxLength={300} disabled={disabled} onChange={(alt) => update({ alt })} />
            <TextInput label={t('fields.caption')} value={item.caption} maxLength={200} disabled={disabled} onChange={(caption) => update({ caption })} />
          </>
        )}
      />
    </div>
  )
}

export function ColumnsForm({ block, record, disabled, onChange }: FormProps<ColumnsBlock>) {
  const t = useModuleTranslation('site-blocks')
  return (
    <div className="blocks-form">
      <TextInput label={t('fields.sectionTitle')} value={block.title} maxLength={160} disabled={disabled} onChange={(title) => onChange({ ...block, title })} />
      <Field label={t('fields.align')}>
        {() => (
          <SegmentedControl<ColumnsBlock['align']>
            aria-label={t('fields.align')}
            value={block.align}
            onValueChange={(align) => onChange({ ...block, align })}
            options={[
              { value: 'center', label: t('fields.alignCenter'), disabled },
              { value: 'left', label: t('fields.alignLeft'), disabled },
            ]}
          />
        )}
      </Field>
      <ItemList
        items={block.items}
        limit={ITEM_LIMITS.columns}
        disabled={disabled}
        itemLabel={(index) => t('items.column', { number: index + 1 })}
        addLabel={t('items.addColumn')}
        create={newColumn}
        onChange={(items) => onChange({ ...block, items })}
        render={(item, update) => (
          <>
            <TextInput label={t('fields.title')} value={item.title} maxLength={120} disabled={disabled} onChange={(title) => update({ title })} />
            <TextAreaInput label={t('fields.text')} description={t('fields.textHelp')} value={item.text} maxLength={2000} disabled={disabled} onChange={(text) => update({ text })} />
            <IconPicker value={item.icon} disabled={disabled} t={t} onChange={(icon) => update({ icon })} />
            <ImageField label={t('fields.columnImage')} record={record} value={item.image} disabled={disabled} onChange={(image) => update({ image })} />
            {item.image && <TextInput label={t('fields.alt')} value={item.alt} maxLength={300} disabled={disabled} onChange={(alt) => update({ alt })} />}
            <TextInput label={t('fields.linkLabel')} value={item.link_label} maxLength={60} disabled={disabled} onChange={(link_label) => update({ link_label })} />
            <Field label={t('fields.link')} description={t('fields.linkHelp')} error={isSafeLink(item.link) ? undefined : t('fields.linkInvalid')}>
              {(control) => <Input {...control} value={item.link} spellCheck={false} autoComplete="off" placeholder="/contato" disabled={disabled} onChange={(event) => update({ link: event.target.value })} />}
            </Field>
          </>
        )}
      />
    </div>
  )
}

export function FaqForm({ block, disabled, onChange }: FormProps<FaqBlock>) {
  const t = useModuleTranslation('site-blocks')
  return (
    <div className="blocks-form">
      <TextInput label={t('fields.sectionTitle')} value={block.title} maxLength={160} disabled={disabled} onChange={(title) => onChange({ ...block, title })} />
      <p className="blocks-muted">{t('fields.faqHelp')}</p>
      <ItemList
        items={block.items}
        limit={ITEM_LIMITS.faq}
        disabled={disabled}
        itemLabel={(index) => t('items.question', { number: index + 1 })}
        addLabel={t('items.addQuestion')}
        create={newFaqItem}
        onChange={(items) => onChange({ ...block, items })}
        render={(item, update) => (
          <>
            <TextInput label={t('fields.question')} value={item.question} maxLength={300} disabled={disabled} onChange={(question) => update({ question })} />
            <TextAreaInput label={t('fields.answer')} description={t('fields.textHelp')} value={item.answer} maxLength={4000} disabled={disabled} onChange={(answer) => update({ answer })} />
          </>
        )}
      />
    </div>
  )
}

export function TestimonialsForm({ block, record, disabled, onChange }: FormProps<TestimonialsBlock>) {
  const t = useModuleTranslation('site-blocks')
  return (
    <div className="blocks-form">
      <TextInput label={t('fields.sectionTitle')} value={block.title} maxLength={160} disabled={disabled} onChange={(title) => onChange({ ...block, title })} />
      <ItemList
        items={block.items}
        limit={ITEM_LIMITS.testimonials}
        disabled={disabled}
        itemLabel={(index) => t('items.testimonial', { number: index + 1 })}
        addLabel={t('items.addTestimonial')}
        create={newTestimonial}
        onChange={(items) => onChange({ ...block, items })}
        render={(item, update) => (
          <>
            <TextAreaInput label={t('fields.quote')} value={item.quote} maxLength={1000} rows={3} disabled={disabled} onChange={(quote) => update({ quote })} />
            <TextInput label={t('fields.personName')} value={item.name} maxLength={120} disabled={disabled} onChange={(name) => update({ name })} />
            <TextInput label={t('fields.personRole')} description={t('fields.personRoleHelp')} value={item.role} maxLength={120} disabled={disabled} onChange={(role) => update({ role })} />
            <ImageField label={t('fields.photo')} record={record} value={item.image} disabled={disabled} onChange={(image) => update({ image })} />
          </>
        )}
      />
    </div>
  )
}

export function ContactForm({ block, disabled, onChange }: FormProps<ContactBlock>) {
  const t = useModuleTranslation('site-blocks')
  const formatting = useRecordFormatting()
  return (
    <div className="blocks-form">
      <TextInput label={t('fields.sectionTitle')} value={block.title} maxLength={160} disabled={disabled} onChange={(title) => onChange({ ...block, title })} />
      <TextAreaInput label={t('fields.contactText')} value={block.text} maxLength={1000} rows={2} disabled={disabled} onChange={(text) => onChange({ ...block, text })} />
      <Field label={t('fields.phone')}>
        {(control) => <PhoneFieldInput id={control.id} aria-describedby={control['aria-describedby']} locale={formatting.locale} fieldLabel={t('fields.phone')} value={block.phone || null} disabled={disabled} onChange={(phone) => onChange({ ...block, phone: phone ?? '' })} />}
      </Field>
      <Field label={t('fields.whatsapp')}>
        {(control) => <PhoneFieldInput id={control.id} aria-describedby={control['aria-describedby']} locale={formatting.locale} fieldLabel={t('fields.whatsapp')} value={block.whatsapp || null} disabled={disabled} onChange={(whatsapp) => onChange({ ...block, whatsapp: whatsapp ?? '' })} />}
      </Field>
      <Field label={t('fields.email')} error={isValidEmail(block.email) ? undefined : t('fields.emailInvalid')}>
        {(control) => <Input {...control} type="email" inputMode="email" autoComplete="off" value={block.email} maxLength={200} disabled={disabled} onChange={(event) => onChange({ ...block, email: event.target.value })} />}
      </Field>
      <TextAreaInput label={t('fields.address')} value={block.address} maxLength={400} rows={2} disabled={disabled} onChange={(address) => onChange({ ...block, address })} />
      <TextAreaInput label={t('fields.hours')} description={t('fields.hoursHelp')} value={block.hours} maxLength={1000} rows={3} disabled={disabled} onChange={(hours) => onChange({ ...block, hours })} />
      <ToggleField type="switch" label={t('fields.showMap')} description={t('fields.showMapHelp')} checked={block.show_map} disabled={disabled || !block.address.trim()} onCheckedChange={(show_map) => onChange({ ...block, show_map })} />
    </div>
  )
}

export function VideoForm({ block, disabled, onChange }: FormProps<VideoBlock>) {
  const t = useModuleTranslation('site-blocks')
  const invalid = block.url.trim() !== '' && !videoOf(block.url)
  return (
    <div className="blocks-form">
      <Field label={t('fields.videoUrl')} description={t('fields.videoUrlHelp')} error={invalid ? t('fields.videoInvalid') : undefined}>
        {(control) => <Input {...control} inputMode="url" spellCheck={false} autoComplete="off" placeholder="https://www.youtube.com/watch?v=…" value={block.url} maxLength={300} disabled={disabled} onChange={(event) => onChange({ ...block, url: event.target.value })} />}
      </Field>
      <TextInput label={t('fields.videoTitle')} description={t('fields.videoTitleHelp')} value={block.title} maxLength={160} disabled={disabled} onChange={(title) => onChange({ ...block, title })} />
      <TextInput label={t('fields.caption')} value={block.caption} maxLength={300} disabled={disabled} onChange={(caption) => onChange({ ...block, caption })} />
    </div>
  )
}

export function EmbedForm({ block, disabled, onChange }: FormProps<EmbedBlock>) {
  const t = useModuleTranslation('site-blocks')
  const found = embedOf(block.url)
  const invalid = block.url.trim() !== '' && !found
  return (
    <div className="blocks-form">
      <Field
        label={t('fields.embedUrl')}
        description={found ? t('fields.embedFound', { provider: t(`providers.${found.provider}`) }) : t('fields.embedUrlHelp')}
        error={invalid ? t('fields.embedInvalid') : undefined}
      >
        {(control) => <Input {...control} inputMode="url" spellCheck={false} autoComplete="off" placeholder="https://docs.google.com/forms/d/e/…/viewform" value={block.url} maxLength={500} disabled={disabled} onChange={(event) => onChange({ ...block, url: event.target.value })} />}
      </Field>
      <TextInput label={t('fields.embedTitle')} description={t('fields.videoTitleHelp')} value={block.title} maxLength={160} disabled={disabled} onChange={(title) => onChange({ ...block, title })} />
      <Field label={t('fields.embedHeight')}>
        {() => (
          <SegmentedControl<EmbedBlock['height']>
            aria-label={t('fields.embedHeight')}
            value={block.height}
            onValueChange={(height) => onChange({ ...block, height })}
            options={[
              { value: 'small', label: t('fields.sizeSmall'), disabled },
              { value: 'medium', label: t('fields.sizeMedium'), disabled },
              { value: 'large', label: t('fields.sizeLarge'), disabled },
            ]}
          />
        )}
      </Field>
    </div>
  )
}
