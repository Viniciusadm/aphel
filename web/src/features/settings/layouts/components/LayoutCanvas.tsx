import { SegmentedControl } from '@/components/ui/segmented-control'
import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  closestCorners,
  useDroppable,
  useSensor,
  useSensors,
  type Announcements,
  type DragEndEvent,
  type UniqueIdentifier,
} from '@dnd-kit/core'
import { SortableContext, sortableKeyboardCoordinates, verticalListSortingStrategy } from '@dnd-kit/sortable'
import { ArrowDown, ArrowLeft, ArrowRight, ArrowUp, Plus, Trash2 } from 'lucide-react'
import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { MAX_LAYOUT_COLUMNS, MAX_LAYOUT_LABEL_LENGTH, MAX_LAYOUT_SECTIONS, MAX_LAYOUT_TABS } from '@/features/records/form/lib/layout'
import type { FormLayout, LayoutContext, LayoutSection } from '@/features/records/form/lib/layout'
import type { FieldDefinition } from '@/features/records/types'
import { cn } from '@/lib/utils'
import {
  addSection,
  addTab,
  isLocked,
  locate,
  moveSection,
  moveTab,
  placeField,
  removeField,
  removeSection,
  removeTab,
  renameSection,
  renameTab,
  setSectionColumns,
  shiftField,
  unplacedFields,
} from '../lib/draft'
import { PALETTE_ID, columnId, parseColumnId } from '../lib/dnd'
import { FieldChip } from './FieldChip'

const SELECT_CLASS =
  'h-8 min-w-0 rounded-md border border-input bg-background px-2 text-sm focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none'

export interface LayoutCanvasProps {
  layout: FormLayout
  candidates: FieldDefinition[]
  context: LayoutContext
  onChange: (layout: FormLayout) => void
  onEditRules: (key: string) => void
}

export function LayoutCanvas({ layout, candidates, context, onChange, onEditRules }: LayoutCanvasProps) {
  const { t } = useTranslation('layouts')
  const [activeTabId, setActiveTabId] = useState<string | undefined>(layout.tabs[0]?.id)
  const activeTab = layout.tabs.find((tab) => tab.id === activeTabId) ?? layout.tabs[0]
  const byKey = new Map(candidates.map((field) => [field.key, field]))
  const available = unplacedFields(layout, candidates)
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  )

  const labelOf = (id: UniqueIdentifier) => byKey.get(String(id))?.label ?? String(id)
  const targetOf = (id: UniqueIdentifier) => {
    const value = String(id)
    if (value === PALETTE_ID || available.some((field) => field.key === value)) return t('reorder.palette')
    const column = parseColumnId(value)
    const location = column ? { sectionId: column.sectionId, column: column.column } : locate(layout, value)
    if (!location) return value
    const section = layout.tabs.flatMap((tab) => tab.sections).find((item) => item.id === location.sectionId)
    return t('reorder.column', { number: location.column + 1, section: section?.label || t('sections.namePlaceholder') })
  }
  const announcements: Announcements = {
    onDragStart: ({ active }) => t('reorder.picked', { label: labelOf(active.id) }),
    onDragOver: ({ active, over }) => (over ? t('reorder.over', { label: labelOf(active.id), target: targetOf(over.id) }) : undefined),
    onDragEnd: ({ active, over }) => (over ? t('reorder.dropped', { label: labelOf(active.id), target: targetOf(over.id) }) : undefined),
    onDragCancel: ({ active }) => t('reorder.cancelled', { label: labelOf(active.id) }),
  }

  const handleDragEnd = ({ active, over }: DragEndEvent) => {
    if (!over) return
    const key = String(active.id)
    const overId = String(over.id)
    if (key === overId) return
    const field = byKey.get(key)
    if (overId === PALETTE_ID || available.some((item) => item.key === overId)) {
      if (field && !isLocked(field, context)) onChange(removeField(layout, key))
      return
    }
    const column = parseColumnId(overId)
    if (column) {
      onChange(placeField(layout, key, column))
      return
    }
    const location = locate(layout, overId)
    if (location) onChange(placeField(layout, key, { sectionId: location.sectionId, column: location.column, index: location.index }))
  }

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCorners}
      onDragEnd={handleDragEnd}
      accessibility={{ announcements }}
    >
      <div className="grid min-w-0 gap-4 lg:grid-cols-[16rem_minmax(0,1fr)]">
        <Palette fields={available} />
        <div className="grid min-w-0 content-start gap-4">
          <div className="flex min-w-0 flex-wrap items-center gap-2" role="group" aria-label={t('tabs.label')}>
            <SegmentedControl
              size="sm"
              value={activeTab?.id ?? ''}
              onValueChange={setActiveTabId}
              options={layout.tabs.map((tab, index) => ({
                value: tab.id,
                label: tab.label || t('tabs.untitled', { number: index + 1 }),
              }))}
            />
            <Button
              type="button"
              size="sm"
              variant="ghost"
              disabled={layout.tabs.length >= MAX_LAYOUT_TABS}
              onClick={() => {
                const next = addTab(layout, t('tabs.newLabel'))
                onChange(next)
                setActiveTabId(next.tabs[next.tabs.length - 1].id)
              }}
            >
              <Plus aria-hidden="true" />
              {t('tabs.add')}
            </Button>
          </div>
          {activeTab && (
            <div className="grid min-w-0 gap-4 rounded-lg border p-3 sm:p-4" data-tab={activeTab.id}>
              <div className="flex min-w-0 flex-col gap-2 sm:flex-row sm:items-end">
                <div className="grid min-w-0 flex-1 gap-1.5">
                  <Label htmlFor={`tab-${activeTab.id}`}>{t('tabs.name')}</Label>
                  <Input
                    id={`tab-${activeTab.id}`}
                    value={activeTab.label ?? ''}
                    maxLength={MAX_LAYOUT_LABEL_LENGTH}
                    placeholder={t('tabs.untitled', { number: layout.tabs.indexOf(activeTab) + 1 })}
                    onChange={(event) => onChange(renameTab(layout, activeTab.id, event.target.value))}
                  />
                </div>
                <div className="flex gap-1">
                  <IconButton label={t('tabs.moveLeft')} onClick={() => onChange(moveTab(layout, activeTab.id, -1))} disabled={layout.tabs[0] === activeTab}>
                    <ArrowLeft aria-hidden="true" />
                  </IconButton>
                  <IconButton
                    label={t('tabs.moveRight')}
                    onClick={() => onChange(moveTab(layout, activeTab.id, 1))}
                    disabled={layout.tabs[layout.tabs.length - 1] === activeTab}
                  >
                    <ArrowRight aria-hidden="true" />
                  </IconButton>
                  <IconButton
                    label={t('tabs.remove')}
                    disabled={layout.tabs.length <= 1}
                    onClick={() => {
                      onChange(removeTab(layout, activeTab.id))
                      setActiveTabId(layout.tabs.find((tab) => tab.id !== activeTab.id)?.id)
                    }}
                  >
                    <Trash2 aria-hidden="true" />
                  </IconButton>
                </div>
              </div>
              {activeTab.sections.map((section, index) => (
                <SectionEditor
                  key={section.id}
                  section={section}
                  first={index === 0}
                  last={index === activeTab.sections.length - 1}
                  removable={activeTab.sections.length > 1}
                  layout={layout}
                  byKey={byKey}
                  available={available}
                  context={context}
                  onChange={onChange}
                  onEditRules={onEditRules}
                />
              ))}
              <Button
                type="button"
                variant="outline"
                className="justify-self-start"
                disabled={activeTab.sections.length >= MAX_LAYOUT_SECTIONS}
                onClick={() => onChange(addSection(layout, activeTab.id, t('sections.newLabel')))}
              >
                <Plus aria-hidden="true" />
                {t('sections.add')}
              </Button>
            </div>
          )}
        </div>
      </div>
    </DndContext>
  )
}

function IconButton({ label, onClick, disabled, children }: { label: string; onClick: () => void; disabled?: boolean; children: ReactNode }) {
  return (
    <Button type="button" variant="ghost" size="icon" aria-label={label} title={label} onClick={onClick} disabled={disabled}>
      {children}
    </Button>
  )
}

function Palette({ fields }: { fields: FieldDefinition[] }) {
  const { t } = useTranslation('layouts')
  const { setNodeRef, isOver } = useDroppable({ id: PALETTE_ID })
  return (
    <section
      ref={setNodeRef}
      aria-labelledby="layout-palette-title"
      className={cn('grid min-w-0 content-start gap-2 rounded-lg border bg-muted/40 p-3', isOver && 'ring-2 ring-ring')}
    >
      <h3 id="layout-palette-title" className="text-sm font-semibold">
        {t('palette.title')}
      </h3>
      <p className="text-xs text-muted-foreground">{t('palette.hint')}</p>
      <SortableContext id={PALETTE_ID} items={fields.map((field) => field.key)} strategy={verticalListSortingStrategy}>
        {fields.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t('palette.empty')}</p>
        ) : (
          <ul className="grid gap-1.5" aria-label={t('palette.title')}>
            {fields.map((field) => (
              <FieldChip key={field.key} field={field} container={PALETTE_ID} />
            ))}
          </ul>
        )}
      </SortableContext>
    </section>
  )
}

interface SectionEditorProps {
  section: LayoutSection
  first: boolean
  last: boolean
  removable: boolean
  layout: FormLayout
  byKey: Map<string, FieldDefinition>
  available: FieldDefinition[]
  context: LayoutContext
  onChange: (layout: FormLayout) => void
  onEditRules: (key: string) => void
}

function SectionEditor({ section, first, last, removable, layout, byKey, available, context, onChange, onEditRules }: SectionEditorProps) {
  const { t } = useTranslation('layouts')
  const nameId = `section-${section.id}`
  return (
    <section aria-label={section.label || t('sections.namePlaceholder')} className="grid min-w-0 gap-3 rounded-md border bg-card p-3" data-section={section.id}>
      <div className="flex min-w-0 flex-col gap-2 sm:flex-row sm:items-end">
        <div className="grid min-w-0 flex-1 gap-1.5">
          <Label htmlFor={nameId}>{t('sections.name')}</Label>
          <Input
            id={nameId}
            value={section.label ?? ''}
            maxLength={MAX_LAYOUT_LABEL_LENGTH}
            placeholder={t('sections.namePlaceholder')}
            onChange={(event) => onChange(renameSection(layout, section.id, event.target.value))}
          />
        </div>
        <div className="flex items-end gap-1">
          <div className="grid gap-1.5">
            <Label htmlFor={`${nameId}-columns`}>{t('sections.columns')}</Label>
            <select
              id={`${nameId}-columns`}
              className={SELECT_CLASS}
              value={section.columns.length}
              onChange={(event) => onChange(setSectionColumns(layout, section.id, Number(event.target.value)))}
            >
              {Array.from({ length: MAX_LAYOUT_COLUMNS }, (_, index) => (
                <option key={index} value={index + 1}>
                  {index + 1}
                </option>
              ))}
            </select>
          </div>
          <IconButton label={t('sections.moveUp')} disabled={first} onClick={() => onChange(moveSection(layout, section.id, -1))}>
            <ArrowUp aria-hidden="true" />
          </IconButton>
          <IconButton label={t('sections.moveDown')} disabled={last} onClick={() => onChange(moveSection(layout, section.id, 1))}>
            <ArrowDown aria-hidden="true" />
          </IconButton>
          <IconButton label={t('sections.remove')} disabled={!removable} onClick={() => onChange(removeSection(layout, section.id))}>
            <Trash2 aria-hidden="true" />
          </IconButton>
        </div>
      </div>
      <div className={cn('grid min-w-0 gap-3', section.columns.length === 2 && 'md:grid-cols-2', section.columns.length === 3 && 'md:grid-cols-3')}>
        {section.columns.map((column, index) => (
          <ColumnEditor
            key={index}
            id={columnId(section.id, index)}
            label={t('sections.columnLabel', { number: index + 1 })}
            fieldKeys={column.fields.filter((rule) => byKey.has(rule.key)).map((rule) => rule.key)}
            onAdd={(key) => onChange(placeField(layout, key, { sectionId: section.id, column: index }))}
            available={available}
          >
            {column.fields.map((rule, position) => {
              const field = byKey.get(rule.key)
              if (!field) return null
              return (
                <FieldChip
                  key={rule.key}
                  field={field}
                  rule={rule}
                  container={columnId(section.id, index)}
                  locked={isLocked(field, context)}
                  onRules={() => onEditRules(rule.key)}
                  onMoveUp={position > 0 ? () => onChange(shiftField(layout, rule.key, -1)) : undefined}
                  onMoveDown={position < column.fields.length - 1 ? () => onChange(shiftField(layout, rule.key, 1)) : undefined}
                  onRemove={() => onChange(removeField(layout, rule.key))}
                />
              )
            })}
          </ColumnEditor>
        ))}
      </div>
    </section>
  )
}

interface ColumnEditorProps {
  id: string
  label: string
  fieldKeys: string[]
  available: FieldDefinition[]
  onAdd: (key: string) => void
  children: ReactNode
}

function ColumnEditor({ id, label, fieldKeys: keys, available, onAdd, children }: ColumnEditorProps) {
  const { t } = useTranslation('layouts')
  const { setNodeRef, isOver } = useDroppable({ id })
  return (
    <div
      ref={setNodeRef}
      data-column={id}
      className={cn('grid min-w-0 content-start gap-2 rounded-md border border-dashed p-2', isOver && 'border-ring bg-accent/40')}
    >
      <span className="text-xs font-medium text-muted-foreground">{label}</span>
      <SortableContext id={id} items={keys} strategy={verticalListSortingStrategy}>
        {keys.length === 0 ? (
          <p className="py-3 text-center text-xs text-muted-foreground">{t('sections.emptyColumn')}</p>
        ) : (
          <ul className="grid gap-1.5" aria-label={label}>
            {children}
          </ul>
        )}
      </SortableContext>
      {available.length > 0 && (
        <select
          aria-label={`${t('fields.add')} — ${label}`}
          className={cn(SELECT_CLASS, 'text-muted-foreground')}
          value=""
          onChange={(event) => {
            if (event.target.value) onAdd(event.target.value)
          }}
        >
          <option value="">{t('fields.add')}</option>
          {available.map((field) => (
            <option key={field.key} value={field.key}>
              {field.label}
            </option>
          ))}
        </select>
      )}
    </div>
  )
}
