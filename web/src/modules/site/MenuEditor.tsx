import { useTranslation } from 'react-i18next'
import { ArrowDown, ArrowUp, CornerDownRight, Plus, Trash2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { isSafeLink } from '@/sdk/blocks'
import type { MenuItem } from './designApi'

function ItemFields({ item, onChange, onRemove, onMove, first, last, child }: { item: MenuItem; onChange: (item: MenuItem) => void; onRemove: () => void; onMove: (offset: number) => void; first: boolean; last: boolean; child?: boolean }) {
  const { t } = useTranslation('site')
  const invalid = item.url !== '' && !isSafeLink(item.url)
  return (
    <div className="site-menu-item" data-child={child || undefined}>
      {child && <CornerDownRight aria-hidden size={16} className="site-menu-child-mark" />}
      <Field label={t('siteEditor.menuLabel')}>{(control) => <Input {...control} value={item.label} maxLength={80} onChange={(event) => onChange({ ...item, label: event.target.value })} />}</Field>
      <Field label={t('siteEditor.menuUrl')} error={invalid ? t('siteEditor.menuUrlInvalid') : undefined}>
        {(control) => <Input {...control} value={item.url} placeholder="/contato" spellCheck={false} autoComplete="off" onChange={(event) => onChange({ ...item, url: event.target.value })} />}
      </Field>
      <div className="site-menu-tools">
        <Button type="button" size="icon" variant="ghost" disabled={first} aria-label={t('siteEditor.moveUp')} title={t('siteEditor.moveUp')} onClick={() => onMove(-1)}>
          <ArrowUp aria-hidden />
        </Button>
        <Button type="button" size="icon" variant="ghost" disabled={last} aria-label={t('siteEditor.moveDown')} title={t('siteEditor.moveDown')} onClick={() => onMove(1)}>
          <ArrowDown aria-hidden />
        </Button>
        <Button type="button" size="icon" variant="ghost" aria-label={t('siteEditor.remove')} title={t('siteEditor.remove')} onClick={onRemove}>
          <Trash2 aria-hidden />
        </Button>
      </div>
    </div>
  )
}

function moved<T>(list: T[], index: number, offset: number): T[] {
  const next = [...list]
  const [item] = next.splice(index, 1)
  next.splice(index + offset, 0, item)
  return next
}

export function MenuEditor({ items, onChange }: { items: MenuItem[]; onChange: (items: MenuItem[]) => void }) {
  const { t } = useTranslation('site')
  const update = (index: number, item: MenuItem) => onChange(items.map((current, position) => (position === index ? item : current)))
  return (
    <section className="site-editor-section">
      <h2>{t('siteEditor.menuItems')}</h2>
      <p className="text-sm text-muted-foreground">{t('siteEditor.menuHelp')}</p>
      <ol className="site-menu-list">
        {items.map((item, index) => (
          <li key={index}>
            <ItemFields item={item} first={index === 0} last={index === items.length - 1} onChange={(next) => update(index, next)} onRemove={() => onChange(items.filter((_, position) => position !== index))} onMove={(offset) => onChange(moved(items, index, offset))} />
            {(item.children ?? []).map((child, childIndex) => (
              <ItemFields
                key={childIndex}
                child
                item={child}
                first={childIndex === 0}
                last={childIndex === (item.children ?? []).length - 1}
                onChange={(next) => update(index, { ...item, children: (item.children ?? []).map((current, position) => (position === childIndex ? next : current)) })}
                onRemove={() => update(index, { ...item, children: (item.children ?? []).filter((_, position) => position !== childIndex) })}
                onMove={(offset) => update(index, { ...item, children: moved(item.children ?? [], childIndex, offset) })}
              />
            ))}
            <Button type="button" size="sm" variant="ghost" className="site-menu-add-child" onClick={() => update(index, { ...item, children: [...(item.children ?? []), { label: '', url: '' }] })}>
              <Plus aria-hidden />
              {t('siteEditor.menuAddChild')}
            </Button>
          </li>
        ))}
      </ol>
      <Button type="button" variant="outline" onClick={() => onChange([...items, { label: '', url: '' }])}>
        <Plus aria-hidden />
        {t('siteEditor.menuAdd')}
      </Button>
    </section>
  )
}
