import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { SegmentedControl } from '@/components/ui/segmented-control'
import { Textarea } from '@/components/ui/textarea'
import { ToggleField } from '@/components/ui/toggle-field'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { Plus, Trash2 } from 'lucide-react'

import { MAX_BUTTONS, isSafeLink, type Block, type ButtonsBlock, type BlockButton } from './blocks'
import { ImageField } from './ImageField'
import { BookingBlockForm, CollectionBlockForm, FormBlockForm } from './ConnectedForms'
import { ColumnsForm, ContactForm, EmbedForm, FaqForm, GalleryForm, TestimonialsForm, VideoForm } from './ItemForms'
import type { BlockRecord } from './api'

interface BlockFormProps {
  block: Block
  record: BlockRecord
  onChange: (block: Block) => void
  disabled?: boolean
}

type Translate = ReturnType<typeof useModuleTranslation>

function LinkField({ label, value, onChange, disabled, t }: { label: string; value: string; onChange: (value: string) => void; disabled?: boolean; t: Translate }) {
  const invalid = !isSafeLink(value)
  return (
    <Field label={label} description={t('fields.linkHelp')} error={invalid ? t('fields.linkInvalid') : undefined}>
      {(control) => (
        <Input
          {...control}
          value={value}
          inputMode="url"
          autoComplete="off"
          spellCheck={false}
          placeholder="/contato"
          disabled={disabled}
          onChange={(event) => onChange(event.target.value)}
        />
      )}
    </Field>
  )
}

function TextField({ label, value, onChange, disabled, maxLength, description }: { label: string; value: string; onChange: (value: string) => void; disabled?: boolean; maxLength: number; description?: string }) {
  return (
    <Field label={label} description={description}>
      {(control) => <Input {...control} value={value} maxLength={maxLength} disabled={disabled} onChange={(event) => onChange(event.target.value)} />}
    </Field>
  )
}

function MarkdownField({ label, value, onChange, disabled, maxLength, t }: { label: string; value: string; onChange: (value: string) => void; disabled?: boolean; maxLength: number; t: Translate }) {
  return (
    <Field label={label} description={t('fields.textHelp')}>
      {(control) => (
        <Textarea {...control} value={value} rows={8} maxLength={maxLength} disabled={disabled} onChange={(event) => onChange(event.target.value)} />
      )}
    </Field>
  )
}

function Choice<T extends string>({ label, value, options, onChange, disabled }: { label: string; value: T; options: { value: T; label: string }[]; onChange: (value: T) => void; disabled?: boolean }) {
  return (
    <Field label={label}>
      {() => <SegmentedControl<T> aria-label={label} value={value} onValueChange={onChange} options={options.map((option) => ({ ...option, disabled }))} />}
    </Field>
  )
}

function ButtonsFields({ block, onChange, disabled, t }: { block: ButtonsBlock; onChange: (block: Block) => void; disabled?: boolean; t: Translate }) {
  const update = (index: number, changes: Partial<BlockButton>) =>
    onChange({ ...block, buttons: block.buttons.map((button, position) => (position === index ? { ...button, ...changes } : button)) })
  return (
    <>
      {block.buttons.map((button, index) => (
        <fieldset key={index} className="blocks-fieldset">
          <legend>{t('fields.button', { number: index + 1 })}</legend>
          <TextField label={t('fields.buttonLabel')} value={button.label} maxLength={60} disabled={disabled} onChange={(label) => update(index, { label })} />
          <LinkField label={t('fields.buttonLink')} value={button.link} disabled={disabled} t={t} onChange={(link) => update(index, { link })} />
          <Choice
            label={t('fields.style')}
            value={button.style}
            disabled={disabled}
            options={[
              { value: 'primary', label: t('fields.stylePrimary') },
              { value: 'secondary', label: t('fields.styleSecondary') },
            ]}
            onChange={(style) => update(index, { style })}
          />
          <Button
            type="button"
            variant="ghost"
            size="sm"
            disabled={disabled}
            onClick={() => onChange({ ...block, buttons: block.buttons.filter((_, position) => position !== index) })}
          >
            <Trash2 aria-hidden />
            {t('fields.removeButton')}
          </Button>
        </fieldset>
      ))}
      {block.buttons.length < MAX_BUTTONS && (
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={disabled}
          onClick={() => onChange({ ...block, buttons: [...block.buttons, { label: '', link: '', style: block.buttons.length ? 'secondary' : 'primary' }] })}
        >
          <Plus aria-hidden />
          {t('fields.addButton')}
        </Button>
      )}
    </>
  )
}

export function BlockForm({ block, record, onChange, disabled }: BlockFormProps) {
  const t = useModuleTranslation('site-blocks')
  const alignments = [
    { value: 'center' as const, label: t('fields.alignCenter') },
    { value: 'left' as const, label: t('fields.alignLeft') },
  ]
  switch (block.type) {
    case 'hero':
      return (
        <div className="blocks-form">
          <TextField label={t('fields.title')} value={block.title} maxLength={160} disabled={disabled} onChange={(title) => onChange({ ...block, title })} />
          <TextField label={t('fields.subtitle')} value={block.subtitle} maxLength={400} disabled={disabled} onChange={(subtitle) => onChange({ ...block, subtitle })} />
          <ImageField record={record} value={block.image} disabled={disabled} onChange={(image) => onChange({ ...block, image })} />
          <TextField label={t('fields.buttonLabel')} value={block.button_label} maxLength={60} disabled={disabled} onChange={(button_label) => onChange({ ...block, button_label })} />
          <LinkField label={t('fields.buttonLink')} value={block.button_link} disabled={disabled} t={t} onChange={(button_link) => onChange({ ...block, button_link })} />
          <Choice label={t('fields.align')} value={block.align} options={alignments} disabled={disabled} onChange={(align) => onChange({ ...block, align })} />
        </div>
      )
    case 'text':
      return (
        <div className="blocks-form">
          <MarkdownField label={t('fields.text')} value={block.text} maxLength={50000} disabled={disabled} t={t} onChange={(text) => onChange({ ...block, text })} />
          <Choice
            label={t('fields.width')}
            value={block.width}
            disabled={disabled}
            options={[
              { value: 'narrow', label: t('fields.widthNarrow') },
              { value: 'wide', label: t('fields.widthWide') },
            ]}
            onChange={(width) => onChange({ ...block, width })}
          />
        </div>
      )
    case 'image':
      return (
        <div className="blocks-form">
          <ImageField record={record} value={block.image} disabled={disabled} onChange={(image) => onChange({ ...block, image })} />
          <TextField label={t('fields.alt')} description={t('fields.altHelp')} value={block.alt} maxLength={300} disabled={disabled} onChange={(alt) => onChange({ ...block, alt })} />
          <TextField label={t('fields.caption')} value={block.caption} maxLength={300} disabled={disabled} onChange={(caption) => onChange({ ...block, caption })} />
          <Choice
            label={t('fields.width')}
            value={block.width}
            disabled={disabled}
            options={[
              { value: 'narrow', label: t('fields.widthNarrow') },
              { value: 'wide', label: t('fields.widthWide') },
              { value: 'full', label: t('fields.widthFull') },
            ]}
            onChange={(width) => onChange({ ...block, width })}
          />
        </div>
      )
    case 'text_image':
      return (
        <div className="blocks-form">
          <TextField label={t('fields.title')} value={block.title} maxLength={160} disabled={disabled} onChange={(title) => onChange({ ...block, title })} />
          <MarkdownField label={t('fields.text')} value={block.text} maxLength={10000} disabled={disabled} t={t} onChange={(text) => onChange({ ...block, text })} />
          <ImageField record={record} value={block.image} disabled={disabled} onChange={(image) => onChange({ ...block, image })} />
          <TextField label={t('fields.alt')} description={t('fields.altHelp')} value={block.alt} maxLength={300} disabled={disabled} onChange={(alt) => onChange({ ...block, alt })} />
          <Choice
            label={t('fields.imageSide')}
            value={block.image_side}
            disabled={disabled}
            options={[
              { value: 'left', label: t('fields.sideLeft') },
              { value: 'right', label: t('fields.sideRight') },
            ]}
            onChange={(image_side) => onChange({ ...block, image_side })}
          />
        </div>
      )
    case 'buttons':
      return (
        <div className="blocks-form">
          <ButtonsFields block={block} disabled={disabled} t={t} onChange={onChange} />
          <Choice label={t('fields.align')} value={block.align} options={alignments} disabled={disabled} onChange={(align) => onChange({ ...block, align })} />
        </div>
      )
    case 'spacer':
      return (
        <div className="blocks-form">
          <Choice
            label={t('fields.size')}
            value={block.size}
            disabled={disabled}
            options={[
              { value: 'small', label: t('fields.sizeSmall') },
              { value: 'medium', label: t('fields.sizeMedium') },
              { value: 'large', label: t('fields.sizeLarge') },
            ]}
            onChange={(size) => onChange({ ...block, size })}
          />
          <ToggleField label={t('fields.divider')} checked={block.divider} disabled={disabled} onCheckedChange={(divider) => onChange({ ...block, divider })} />
        </div>
      )
    case 'gallery':
      return <GalleryForm block={block} record={record} disabled={disabled} onChange={onChange} />
    case 'columns':
      return <ColumnsForm block={block} record={record} disabled={disabled} onChange={onChange} />
    case 'faq':
      return <FaqForm block={block} record={record} disabled={disabled} onChange={onChange} />
    case 'testimonials':
      return <TestimonialsForm block={block} record={record} disabled={disabled} onChange={onChange} />
    case 'contact':
      return <ContactForm block={block} record={record} disabled={disabled} onChange={onChange} />
    case 'video':
      return <VideoForm block={block} record={record} disabled={disabled} onChange={onChange} />
    case 'form':
      return <FormBlockForm block={block} disabled={disabled} onChange={onChange} />
    case 'booking':
      return <BookingBlockForm block={block} disabled={disabled} onChange={onChange} />
    case 'collection':
      return <CollectionBlockForm block={block} disabled={disabled} onChange={onChange} />
    case 'embed':
      return <EmbedForm block={block} record={record} disabled={disabled} onChange={onChange} />
  }
}
