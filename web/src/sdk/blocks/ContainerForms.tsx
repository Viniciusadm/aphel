import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { NativeSelect } from '@/components/ui/native-select'
import { ToggleField } from '@/components/ui/toggle-field'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { ArrowDown, ArrowUp, ChevronRight, Copy, Trash2 } from 'lucide-react'
import { createElement, useState } from 'react'

import { AddBlock } from './AddBlock'
import { BlockForm } from './BlockForm'
import { blockIcon } from './blockIcons'
import { useBlockScope } from './scope'
import { MAX_DEPTH, blockProblems, blockSummary, duplicateBlock, newBlock, type Block, type ContainerBlock, type GridLayout } from './blocks'
import { ImageField } from './ImageField'
import { QueryFields } from './SiteForms'
import type { BlockRecord } from './api'

type Translate = ReturnType<typeof useModuleTranslation>

function Options<T extends string>({ label, value, options, onChange, disabled }: { label: string; value: T; options: { value: T; label: string }[]; onChange: (value: T) => void; disabled?: boolean }) {
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

const GRID_LAYOUTS: GridLayout[] = ['1-1', '1-1-1', '1-1-1-1', '2-1', '1-2', '1-2-1', '3-2', '2-3']

function GridPreview({ layout }: { layout: GridLayout }) {
  return (
    <span className="blocks-grid-preview" aria-hidden="true">
      {layout.split('-').map((part, index) => (
        <span key={index} style={{ flexGrow: Number(part) }} />
      ))}
    </span>
  )
}

function ContainerFields({ block, record, onChange, disabled, t }: { block: ContainerBlock; record: BlockRecord; onChange: (block: Block) => void; disabled?: boolean; t: Translate }) {
  const option =
    (prefix: string) =>
    <T extends string>(value: T) => ({ value, label: t(`container.${prefix}_${value.replace(/-/g, '_')}`) })
  switch (block.type) {
    case 'group':
      return <Options label={t('container.innerWidth')} value={block.inner_width} disabled={disabled} options={(['full', 'content', 'wide'] as const).map(option('inner'))} onChange={(inner_width) => onChange({ ...block, inner_width })} />
    case 'row':
      return (
        <>
          <Options label={t('container.justify')} value={block.justify} disabled={disabled} options={(['start', 'center', 'end', 'between'] as const).map(option('justify'))} onChange={(justify) => onChange({ ...block, justify })} />
          <Options label={t('container.vertical')} value={block.vertical} disabled={disabled} options={(['center', 'top', 'bottom', 'stretch'] as const).map(option('vertical'))} onChange={(vertical) => onChange({ ...block, vertical })} />
          <ToggleField label={t('container.wrap')} checked={block.wrap} disabled={disabled} onCheckedChange={(wrap) => onChange({ ...block, wrap })} />
        </>
      )
    case 'stack':
      return <Options label={t('container.alignItems')} value={block.align_items} disabled={disabled} options={(['stretch', 'start', 'center', 'end'] as const).map(option('align'))} onChange={(align_items) => onChange({ ...block, align_items })} />
    case 'grid':
      return (
        <>
          <Field label={t('container.layout')}>
            {() => (
              <div className="blocks-grid-layouts" role="radiogroup" aria-label={t('container.layout')}>
                {GRID_LAYOUTS.map((layout) => (
                  <button key={layout} type="button" role="radio" aria-checked={block.layout === layout} aria-label={layout} title={layout} className="blocks-grid-layout" disabled={disabled} onClick={() => onChange({ ...block, layout })}>
                    <GridPreview layout={layout} />
                  </button>
                ))}
              </div>
            )}
          </Field>
          <Options label={t('container.vertical')} value={block.vertical} disabled={disabled} options={(['stretch', 'center', 'top', 'bottom'] as const).map(option('vertical'))} onChange={(vertical) => onChange({ ...block, vertical })} />
          <Options label={t('container.stackOn')} value={block.stack_on} disabled={disabled} options={(['mobile', 'tablet', 'never'] as const).map(option('stack'))} onChange={(stack_on) => onChange({ ...block, stack_on })} />
        </>
      )
    case 'cover':
      return (
        <>
          <ImageField record={record} value={block.image} disabled={disabled} onChange={(image) => onChange({ ...block, image })} />
          <Options label={t('container.minHeight')} value={block.min_height} disabled={disabled} options={(['small', 'medium', 'large', 'screen'] as const).map(option('height'))} onChange={(min_height) => onChange({ ...block, min_height })} />
          <Options label={t('container.position')} value={block.position} disabled={disabled} options={(['center', 'center-left', 'bottom-left', 'bottom-center', 'top-left'] as const).map(option('position'))} onChange={(position) => onChange({ ...block, position })} />
          <Options label={t('container.overlay')} value={block.overlay} disabled={disabled} options={(['0', '20', '40', '60', '80'] as const).map((value) => ({ value, label: `${value}%` }))} onChange={(overlay) => onChange({ ...block, overlay })} />
          <ToggleField label={t('container.darkText')} checked={block.dark_text} disabled={disabled} onCheckedChange={(dark_text) => onChange({ ...block, dark_text })} />
        </>
      )
    case 'query':
      return <QueryFields block={block} disabled={disabled} onChange={onChange} />
    case 'details':
      return (
        <>
          <Field label={t('container.summary')}>
            {(control) => <Input {...control} value={block.summary} maxLength={300} disabled={disabled} onChange={(event) => onChange({ ...block, summary: event.target.value })} />}
          </Field>
          <ToggleField label={t('container.open')} checked={block.open} disabled={disabled} onCheckedChange={(open) => onChange({ ...block, open })} />
        </>
      )
  }
}

function Child({ block, index, count, depth, record, disabled, onChange, onMove, onDuplicate, onRemove, t }: { block: Block; index: number; count: number; depth: number; record: BlockRecord; disabled?: boolean; onChange: (block: Block) => void; onMove: (offset: number) => void; onDuplicate: () => void; onRemove: () => void; t: Translate }) {
  const [open, setOpen] = useState(false)
  const problems = blockProblems(block)
  return (
    <li className="blocks-child" data-open={open || undefined}>
      <div className="blocks-child-row">
        <button type="button" className="blocks-child-toggle" aria-expanded={open} onClick={() => setOpen(!open)}>
          <ChevronRight aria-hidden size={16} className="blocks-child-chevron" />
          {createElement(blockIcon(block.type), { 'aria-hidden': true, size: 16 })}
          <strong>{t(`types.${block.type}`)}</strong>
          <span className="blocks-child-summary">{blockSummary(block)}</span>
          {problems.length > 0 && <span className="blocks-child-problem">{t('container.needsAttention')}</span>}
        </button>
        <span className="blocks-child-tools">
          <Button type="button" variant="ghost" size="icon" disabled={disabled || index === 0} aria-label={t('container.moveUp')} title={t('container.moveUp')} onClick={() => onMove(-1)}>
            <ArrowUp aria-hidden />
          </Button>
          <Button type="button" variant="ghost" size="icon" disabled={disabled || index === count - 1} aria-label={t('container.moveDown')} title={t('container.moveDown')} onClick={() => onMove(1)}>
            <ArrowDown aria-hidden />
          </Button>
          <Button type="button" variant="ghost" size="icon" disabled={disabled} aria-label={t('container.duplicate')} title={t('container.duplicate')} onClick={onDuplicate}>
            <Copy aria-hidden />
          </Button>
          <Button type="button" variant="ghost" size="icon" disabled={disabled} aria-label={t('container.remove')} title={t('container.remove')} onClick={onRemove}>
            <Trash2 aria-hidden />
          </Button>
        </span>
      </div>
      {open && (
        <div className="blocks-child-body">
          <BlockForm block={block} record={record} disabled={disabled} depth={depth} onChange={onChange} />
        </div>
      )}
    </li>
  )
}

export function ContainerForm({ block, record, onChange, disabled, depth }: { block: ContainerBlock; record: BlockRecord; onChange: (block: Block) => void; disabled?: boolean; depth: number }) {
  const t = useModuleTranslation('site-blocks')
  const scope = useBlockScope()
  const children = block.children
  const update = (next: Block[]) => onChange({ ...block, children: next })
  const move = (index: number, offset: number) => {
    const next = [...children]
    const [item] = next.splice(index, 1)
    next.splice(index + offset, 0, item)
    update(next)
  }
  return (
    <div className="blocks-form">
      <ContainerFields block={block} record={record} onChange={onChange} disabled={disabled} t={t} />
      <Field label={t('container.children')} description={children.length === 0 ? t('container.empty') : undefined}>
        {() => (
          <div className="blocks-children">
            {children.length > 0 && (
              <ol className="blocks-child-list">
                {children.map((child, index) => (
                  <Child
                    key={child.id}
                    block={child}
                    index={index}
                    count={children.length}
                    depth={depth + 1}
                    record={record}
                    disabled={disabled}
                    t={t}
                    onChange={(changed) => update(children.map((item, position) => (position === index ? changed : item)))}
                    onMove={(offset) => move(index, offset)}
                    onDuplicate={() => update([...children.slice(0, index + 1), duplicateBlock(child), ...children.slice(index + 1)])}
                    onRemove={() => update(children.filter((_, position) => position !== index))}
                  />
                ))}
              </ol>
            )}
            <AddBlock disabled={disabled} scope={scope} allowContainers={depth + 2 < MAX_DEPTH} onAdd={(type) => update([...children, newBlock(type)])} onAddBlock={(created) => update([...children, created])} />
          </div>
        )}
      </Field>
    </div>
  )
}
