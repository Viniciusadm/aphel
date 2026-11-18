import './siteEditor.css'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link } from 'react-router'
import { ArrowLeft, Download, Laptop, Monitor, Plus, Redo2, RotateCcw, Send, Smartphone, Tablet, Undo2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ConfirmDialog } from '@/components/ui/confirm-dialog'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { NativeSelect } from '@/components/ui/native-select'
import { SegmentedControl } from '@/components/ui/segmented-control'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { notifyError, notifySuccess } from '@/components/feedback'
import { isApiError } from '@/api'
import { API_BASE_URL } from '@/api/client'
import {
  BlockForm,
  BlockScopeContext,
  duplicateBlock,
  duplicateInPlace,
  findBlock,
  insertBlock,
  isSafeLink,
  isContainerBlock,
  locateBlock,
  moveBlock,
  removeBlock,
  replaceBlock,
  useBlocksValidation,
  type Block,
} from '@/sdk/blocks'
import { localized, usePluginBlocks } from '@/sdk/blocks/api'
import { BlockOutline } from './BlockOutline'
import { DesignCanvas, type CanvasDevice } from './DesignCanvas'
import { MenuEditor } from './MenuEditor'
import { designApi, designKeys, parseContent, parseMenu, type DesignCatalog, type DesignContent, type DesignKind, type DesignRecord, type MenuItem, type SiteTarget, type TemplateEntry } from './designApi'

interface DesignEntry {
  target: string
  kind: DesignKind
  label: string
  group: string
  fallback: DesignContent | null
  template?: TemplateEntry
}

function keyOf(target: string) {
  return target.slice(target.indexOf(':') + 1)
}

function slugify(text: string) {
  return text
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 48)
}

function entriesOf(catalog: DesignCatalog, records: DesignRecord[], pageLabel: (page: string) => string, t: (key: string) => string): DesignEntry[] {
  const entries: DesignEntry[] = []
  for (const template of catalog.templates) {
    const group = template.owner_name
    entries.push({ target: `template:${template.key}`, kind: 'template', label: pageLabel(template.page), group, fallback: template.default, template })
    for (const variant of template.variants) entries.push({ target: `template:${template.key}@${variant.variant}`, kind: 'template', label: `${pageLabel(template.page)} · ${variant.variant}`, group, fallback: variant.default, template })
  }
  for (const record of records.filter((record) => record.kind === 'template' && !entries.some((entry) => entry.target === record.target))) {
    const base = keyOf(record.target).split('@')[0]
    const template = catalog.templates.find((candidate) => candidate.key === base)
    entries.push({ target: record.target, kind: 'template', label: record.name || keyOf(record.target), group: template?.owner_name ?? t('siteEditor.templates'), fallback: template?.default ?? null, template })
  }
  const partKeys = new Set([...Object.keys(catalog.parts), ...records.filter((record) => record.kind === 'part').map((record) => keyOf(record.target))])
  for (const key of partKeys) entries.push({ target: `part:${key}`, kind: 'part', label: records.find((record) => record.target === `part:${key}`)?.name || (key === 'header' || key === 'footer' ? t(`siteEditor.part_${key}`) : key), group: t('siteEditor.parts'), fallback: catalog.parts[key] ?? null })
  const patternKeys = new Set([...Object.keys(catalog.patterns), ...records.filter((record) => record.kind === 'pattern').map((record) => keyOf(record.target))])
  for (const key of patternKeys) entries.push({ target: `pattern:${key}`, kind: 'pattern', label: records.find((record) => record.target === `pattern:${key}`)?.name || catalog.patterns[key]?.name || key, group: t('siteEditor.patterns'), fallback: catalog.patterns[key] ?? null })
  for (const record of records.filter((record) => record.kind === 'menu')) entries.push({ target: record.target, kind: 'menu', label: record.name || keyOf(record.target), group: t('siteEditor.menus'), fallback: null })
  return entries
}

function suggestedPath(entry: DesignEntry, targets: SiteTarget[]): string {
  const template = entry.template
  if (!template) return '/'
  const group = template.prefixed && template.listing ? template.prefix_entity : template.entity
  const found = group ? targets.find((target) => target.target.startsWith(`record:${group}:`)) : undefined
  if (found) return found.path
  return template.path.includes('{') ? '/' : template.path
}

export function SiteEditorPage() {
  const { t } = useTranslation('site')
  const client = useQueryClient()
  const validation = useBlocksValidation()
  const catalog = useQuery({ queryKey: designKeys.catalog, queryFn: designApi.catalog })
  const records = useQuery({ queryKey: designKeys.records, queryFn: designApi.records })
  const targets = useQuery({ queryKey: designKeys.targets, queryFn: designApi.targets })
  const [current, setCurrent] = useState<string | null>(null)
  const [creating, setCreating] = useState<Exclude<DesignKind, 'font'> | null>(null)
  const [name, setName] = useState('')
  const [base, setBase] = useState('')
  const pages = t('siteEditor.pages', { returnObjects: true }) as Record<string, string>
  const pageLabel = useCallback((page: string) => pages[page] ?? page, [pages])
  const entries = useMemo(() => (catalog.data && records.data ? entriesOf(catalog.data, records.data, pageLabel, t as (key: string) => string) : []), [catalog.data, records.data, pageLabel, t])
  if (catalog.isPending || records.isPending || targets.isPending || validation.isPending) return <Skeleton className="m-6 h-96" />
  if (!catalog.data || !records.data) return <ErrorState onRetry={() => { void catalog.refetch(); void records.refetch() }} />
  const selected = entries.find((entry) => entry.target === current) ?? entries.find((entry) => entry.target.endsWith(':post')) ?? entries[0]
  const groups = [...new Set(entries.map((entry) => entry.group))]

  async function create() {
    if (!creating || !name.trim()) return
    const key = slugify(name) || `item-${Date.now()}`
    const template = catalog.data?.templates.find((candidate) => candidate.key === (base || catalog.data?.templates[0]?.key))
    if (creating === 'template' && !template) return
    const target = creating === 'template' && template ? `template:${template.key}@${key}` : `${creating}:${key}`
    const initial = creating === 'menu' ? '[]' : JSON.stringify(creating === 'template' ? template?.default ?? { blocks: [] } : { blocks: [] })
    try {
      const record = await designApi.create({ target, kind: creating, name: name.trim(), draft_content: initial })
      await client.invalidateQueries({ queryKey: designKeys.records })
      setCurrent(record.target)
      setCreating(null)
      setName('')
    } catch (error) {
      notifyError(error)
    }
  }

  return (
    <div className="site-editor">
      <aside className="site-editor-nav" aria-label={t('siteEditor.designs')}>
        <Link to="/site/overview" className="site-editor-back">
          <ArrowLeft aria-hidden size={16} />
          {t('siteEditor.back')}
        </Link>
        <h1>{t('siteEditor.title')}</h1>
        {groups.map((group) => (
          <section key={group}>
            <h2>{group}</h2>
            <ul>
              {entries
                .filter((entry) => entry.group === group)
                .map((entry) => {
                  const record = records.data.find((item) => item.target === entry.target)
                  return (
                    <li key={entry.target}>
                      <button type="button" aria-current={selected?.target === entry.target || undefined} onClick={() => setCurrent(entry.target)}>
                        <span>{entry.label}</span>
                        {record && <small data-state={record.publication_state}>{t(`siteEditor.state_${record.publication_state === 'published' ? 'custom' : 'draft'}`)}</small>}
                      </button>
                    </li>
                  )
                })}
            </ul>
          </section>
        ))}
        <section>
          <h2>{t('siteEditor.share')}</h2>
          <div className="site-editor-create-buttons">
            <Button asChild size="sm" variant="outline">
              <a href={`${API_BASE_URL}/v1/site/designs/export`} download>
                <Download aria-hidden />
                {t('siteEditor.export')}
              </a>
            </Button>
            <p className="px-0.5 text-xs text-muted-foreground">{t('siteEditor.exportHelp')}</p>
          </div>
        </section>
        <section>
          <h2>{t('siteEditor.create')}</h2>
          {creating ? (
            <form
              className="site-editor-create"
              onSubmit={(event) => {
                event.preventDefault()
                void create()
              }}
            >
              {creating === 'template' && (
                <Field label={t('siteEditor.basePage')} description={t('siteEditor.basePageHelp')}>
                  {(control) => (
                    <NativeSelect {...control} className="w-full" value={base || catalog.data.templates[0]?.key} onValueChange={setBase}>
                      {catalog.data.templates.map((template) => (
                        <option key={template.key} value={template.key}>
                          {template.owner_name} · {pageLabel(template.page)}
                        </option>
                      ))}
                    </NativeSelect>
                  )}
                </Field>
              )}
              <Field label={creating === 'template' ? t('siteEditor.variantName') : t(`siteEditor.new_${creating}`)} description={creating === 'template' ? t('siteEditor.variantHelp') : undefined}>{(control) => <Input {...control} autoFocus value={name} maxLength={80} onChange={(event) => setName(event.target.value)} />}</Field>
              <div className="flex gap-2">
                <Button type="submit" size="sm" disabled={!name.trim()}>
                  {t('siteEditor.createSubmit')}
                </Button>
                <Button type="button" size="sm" variant="ghost" onClick={() => setCreating(null)}>
                  {t('siteEditor.cancel')}
                </Button>
              </div>
            </form>
          ) : (
            <div className="site-editor-create-buttons">
              {(['template', 'pattern', 'part', 'menu'] as const).map((kind) => (
                <Button key={kind} type="button" size="sm" variant="outline" onClick={() => setCreating(kind)}>
                  <Plus aria-hidden />
                  {t(`siteEditor.new_${kind}`)}
                </Button>
              ))}
            </div>
          )}
        </section>
      </aside>
      {selected ? (
        <DesignWorkspace
          key={selected.target}
          entry={selected}
          record={records.data.find((record) => record.target === selected.target) ?? null}
          defaultPath={suggestedPath(selected, targets.data ?? [])}
          patterns={{ ...catalog.data.patterns, ...Object.fromEntries(records.data.filter((record) => record.kind === 'pattern').map((record) => [keyOf(record.target), parseContent(record.draft_content ?? record.content)])) }}
          patternNames={Object.fromEntries(records.data.filter((record) => record.kind === 'pattern').map((record) => [keyOf(record.target), record.name ?? keyOf(record.target)]))}
          onRecordsChanged={() => client.invalidateQueries({ queryKey: designKeys.records })}
        />
      ) : (
        <p className="p-6 text-muted-foreground">{t('siteEditor.empty')}</p>
      )}
    </div>
  )
}

type History = { past: DesignContent[]; future: DesignContent[] }

function completeMenu(items: MenuItem[]): MenuItem[] {
  return items
    .filter((item) => item.label.trim() && item.url.trim() && isSafeLink(item.url.trim()))
    .map((item) => ({ ...item, children: completeMenu(item.children ?? []) }))
}

function DesignWorkspace({ entry, record: initialRecord, defaultPath, onRecordsChanged, patterns, patternNames }: { entry: DesignEntry; record: DesignRecord | null; defaultPath: string; onRecordsChanged: () => Promise<void>; patterns: Record<string, DesignContent>; patternNames: Record<string, string> }) {
  const { t, i18n } = useTranslation('site')
  const plugins = usePluginBlocks()
  const [record, setRecord] = useState<DesignRecord | null>(initialRecord)
  const [content, setContent] = useState<DesignContent>(() => (initialRecord ? parseContent(initialRecord.draft_content ?? initialRecord.content) : entry.fallback ?? { blocks: [] }))
  const [menu, setMenu] = useState<MenuItem[]>(() => parseMenu(initialRecord?.draft_content ?? initialRecord?.content))
  const [history, setHistory] = useState<History>({ past: [], future: [] })
  const [selected, setSelected] = useState<string | null>(null)
  const [device, setDevice] = useState<CanvasDevice>('desktop')
  const [path, setPath] = useState(defaultPath)
  const [html, setHtml] = useState('')
  const [loading, setLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [dirty, setDirty] = useState(false)
  const [resetting, setResetting] = useState(false)
  const [patternName, setPatternName] = useState('')
  const saving = useRef<Promise<DesignRecord | null> | null>(null)
  const menuKind = entry.kind === 'menu'
  const serialized = menuKind ? JSON.stringify(completeMenu(menu)) : JSON.stringify(content)

  const persist = useCallback(
    async (text: string): Promise<DesignRecord | null> => {
      const run = async () => {
        if (record) {
          const saved = await designApi.save(record, { draft_content: text })
          setRecord(saved)
          return saved
        }
        const created = await designApi.create({ target: entry.target, kind: entry.kind, name: entry.label, draft_content: text })
        setRecord(created)
        await onRecordsChanged()
        return created
      }
      saving.current = run()
      try {
        return await saving.current
      } finally {
        saving.current = null
      }
    },
    [record, entry, onRecordsChanged],
  )

  useEffect(() => {
    if (!dirty) return
    const timer = window.setTimeout(() => {
      persist(serialized)
        .then(() => setDirty(false))
        .catch(notifyError)
    }, 1200)
    return () => window.clearTimeout(timer)
  }, [dirty, serialized, persist])

  useEffect(() => {
    let cancelled = false
    const timer = window.setTimeout(() => {
      setLoading(true)
      designApi
        .preview(entry.target, serialized, path || '/')
        .then((next) => {
          if (!cancelled) setHtml(next)
        })
        .catch((error) => {
          if (cancelled) return
          const message = isApiError(error) && error.status === 404 ? t('page.noPages') : String((error as Error).message ?? error)
          const note = document.createElement('p')
          note.style.cssText = 'font:16px system-ui;padding:2rem;color:#b42318'
          note.textContent = message
          setHtml(note.outerHTML)
        })
        .finally(() => {
          if (!cancelled) setLoading(false)
        })
    }, 450)
    return () => {
      cancelled = true
      window.clearTimeout(timer)
    }
  }, [entry.target, serialized, path, t])

  const change = useCallback((next: DesignContent) => {
    setHistory((current) => ({ past: [...current.past.slice(-80), content], future: [] }))
    setContent(next)
    setDirty(true)
  }, [content])
  const changeBlocks = useCallback((blocks: Block[]) => change({ ...content, blocks }), [change, content])

  const undo = useCallback(() => {
    setHistory((current) => {
      const previous = current.past.at(-1)
      if (!previous) return current
      setContent(previous)
      setDirty(true)
      return { past: current.past.slice(0, -1), future: [content, ...current.future] }
    })
  }, [content])
  const redo = useCallback(() => {
    setHistory((current) => {
      const next = current.future[0]
      if (!next) return current
      setContent(next)
      setDirty(true)
      return { past: [...current.past, content], future: current.future.slice(1) }
    })
  }, [content])

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null
      const typing = target?.closest('input, textarea, select, [contenteditable="true"]')
      const mod = event.ctrlKey || event.metaKey
      if (mod && event.key.toLowerCase() === 'z' && !typing) {
        event.preventDefault()
        if (event.shiftKey) redo()
        else undo()
        return
      }
      if (mod && event.key.toLowerCase() === 'y' && !typing) {
        event.preventDefault()
        redo()
        return
      }
      if (!selected || typing || menuKind) return
      if (event.key === 'Delete') {
        event.preventDefault()
        changeBlocks(removeBlock(content.blocks, selected))
        setSelected(null)
      } else if (event.altKey && (event.key === 'ArrowUp' || event.key === 'ArrowDown')) {
        event.preventDefault()
        changeBlocks(moveBlock(content.blocks, selected, event.key === 'ArrowUp' ? -1 : 1))
      } else if (mod && event.key.toLowerCase() === 'd') {
        event.preventDefault()
        const result = duplicateInPlace(content.blocks, selected)
        changeBlocks(result.blocks)
        if (result.copy) setSelected(result.copy.id)
      } else if (event.key === 'Escape') {
        const location = locateBlock(content.blocks, selected)
        setSelected(location?.parent ?? null)
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [selected, content, changeBlocks, undo, redo, menuKind])

  const editable = useMemo(() => {
    const ids = new Set<string>()
    const walk = (blocks: Block[]) => blocks.forEach((block) => {
      ids.add(block.id)
      if (isContainerBlock(block)) walk(block.children)
    })
    walk(content.blocks)
    return ids
  }, [content.blocks])

  async function publish() {
    setBusy(true)
    try {
      const pending = saving.current ? await saving.current : null
      const latest = dirty || !record ? await persist(serialized) : pending ?? record
      setDirty(false)
      if (!latest) return
      const request = await designApi.publish(latest)
      if (['blocked', 'failed', 'cancelled', 'superseded'].includes(request.status)) notifyError(t('siteEditor.publishFailed'))
      else notifySuccess(request.status === 'completed' ? t('siteEditor.published') : t('siteEditor.pending'))
      setRecord(await designApi.load(latest.id))
      await onRecordsChanged()
    } catch (error) {
      notifyError(error)
    } finally {
      setBusy(false)
    }
  }

  async function reset() {
    if (!record) return
    try {
      await designApi.remove(record)
      setRecord(null)
      setContent(entry.fallback ?? { blocks: [] })
      setMenu([])
      setHistory({ past: [], future: [] })
      setDirty(false)
      await onRecordsChanged()
      notifySuccess(t('siteEditor.resetDone'))
    } catch (error) {
      notifyError(error)
    }
  }

  const block = selected ? findBlock(content.blocks, selected) : null
  const pluginLabel = (type: string) => {
    const definition = plugins.data?.find((candidate) => candidate.type === type)
    return definition ? localized(definition.label, i18n.language) || type : null
  }
  const published = record?.publication_state === 'published'
  const changed = Boolean(record && record.draft_content !== record.content)
  return (
    <>
      <header className="site-editor-bar">
        <div className="site-editor-title">
          <strong>{entry.label}</strong>
          <small>{record ? (published && !changed && !dirty ? t('siteEditor.statusPublished') : t('siteEditor.statusDraft')) : entry.fallback ? t('siteEditor.statusDefault') : t('siteEditor.statusEmpty')}</small>
        </div>
        <div className="site-editor-actions">
          {!menuKind && (
            <>
              <Button type="button" size="icon" variant="ghost" disabled={history.past.length === 0} aria-label={t('siteEditor.undo')} title={t('siteEditor.undo')} onClick={undo}>
                <Undo2 aria-hidden />
              </Button>
              <Button type="button" size="icon" variant="ghost" disabled={history.future.length === 0} aria-label={t('siteEditor.redo')} title={t('siteEditor.redo')} onClick={redo}>
                <Redo2 aria-hidden />
              </Button>
            </>
          )}
          <SegmentedControl<CanvasDevice>
            aria-label={t('siteEditor.device')}
            size="sm"
            value={device}
            onValueChange={setDevice}
            options={[
              { value: 'desktop', label: <Monitor aria-label={t('siteEditor.desktop')} size={16} /> },
              { value: 'tablet', label: <Tablet aria-label={t('siteEditor.tablet')} size={16} /> },
              { value: 'mobile', label: <Smartphone aria-label={t('siteEditor.mobile')} size={16} /> },
            ]}
          />
          <label className="site-editor-path">
            <Laptop aria-hidden size={14} />
            <span className="sr-only">{t('siteEditor.path')}</span>
            <input value={path} onChange={(event) => setPath(event.target.value)} spellCheck={false} />
          </label>
          {record && (
            <Button type="button" variant="ghost" disabled={busy} onClick={() => setResetting(true)}>
              <RotateCcw aria-hidden />
              {entry.fallback ? t('siteEditor.reset') : t('siteEditor.delete')}
            </Button>
          )}
          <Button type="button" disabled={busy} onClick={() => void publish()}>
            <Send aria-hidden />
            {t('siteEditor.publish')}
          </Button>
        </div>
      </header>
      <main className="site-editor-stage">
        <DesignCanvas html={html} device={device} editable={editable} selected={selected} onSelect={setSelected} loading={loading} />
      </main>
      <aside className="site-editor-inspector" aria-label={t('siteEditor.inspector')}>
        <BlockScopeContext.Provider value="template">
          {menuKind ? (
            <MenuEditor items={menu} onChange={(items) => { setMenu(items); setDirty(true) }} />
          ) : (
            <>
              <section className="site-editor-section">
                <h2>{t('siteEditor.structure')}</h2>
                <BlockOutline
                  blocks={content.blocks}
                  selected={selected}
                  disabled={busy}
                  pluginLabel={pluginLabel}
                  onSelect={setSelected}
                  onMove={(id, offset) => changeBlocks(moveBlock(content.blocks, id, offset))}
                  onDuplicate={(id) => {
                    const result = duplicateInPlace(content.blocks, id)
                    changeBlocks(result.blocks)
                    if (result.copy) setSelected(result.copy.id)
                  }}
                  onRemove={(id) => {
                    changeBlocks(removeBlock(content.blocks, id))
                    setSelected(null)
                  }}
                  onInsert={(parent, index, created) => {
                    changeBlocks(insertBlock(content.blocks, parent, index, created))
                    setSelected(created.id)
                  }}
                />
              </section>
              {block ? (
                <>
                  <section className="site-editor-section">
                    <h2>{pluginLabel(block.type) ?? t('siteEditor.block')}</h2>
                    <BlockForm key={block.id} block={block} record={{ recordType: 'site.design', recordId: record?.id ?? '' }} disabled={busy} onChange={(next) => changeBlocks(replaceBlock(content.blocks, next))} />
                  </section>
                  <form
                    className="site-editor-section"
                    onSubmit={(event) => {
                      event.preventDefault()
                      const label = patternName.trim()
                      if (!label) return
                      designApi
                        .create({ target: `pattern:${slugify(label) || `pattern-${Date.now()}`}`, kind: 'pattern', name: label, draft_content: JSON.stringify({ blocks: [block] }) })
                        .then(async () => {
                          setPatternName('')
                          await onRecordsChanged()
                          notifySuccess(t('siteEditor.patternSaved'))
                        })
                        .catch(notifyError)
                    }}
                  >
                    <h2>{t('siteEditor.saveAsPattern')}</h2>
                    <Field label={t('siteEditor.patternName')} description={t('siteEditor.saveAsPatternHelp')}>
                      {(control) => <Input {...control} value={patternName} maxLength={80} onChange={(event) => setPatternName(event.target.value)} />}
                    </Field>
                    <Button type="submit" size="sm" variant="outline" className="justify-self-start" disabled={!patternName.trim()}>
                      {t('siteEditor.saveAsPatternSubmit')}
                    </Button>
                  </form>
                </>
              ) : (
                entry.kind === 'template' && (
                  <section className="site-editor-section">
                    <h2>{t('siteEditor.templateSettings')}</h2>
                    {(['header', 'footer'] as const).map((area) => (
                      <Field key={area} label={t(`siteEditor.area_${area}`)}>
                        {(control) => (
                          <NativeSelect {...control} className="w-full" value={content[area] ?? ''} onValueChange={(value) => change({ ...content, [area]: value || undefined })}>
                            <option value="">{t(`siteEditor.part_${area}`)}</option>
                            <option value="none">{t('siteEditor.noPart')}</option>
                          </NativeSelect>
                        )}
                      </Field>
                    ))}
                    <Field label={t('siteEditor.templateCss')} description={t('siteEditor.templateCssHelp')}>
                      {(control) => <Textarea {...control} rows={6} spellCheck={false} className="font-mono text-xs" value={content.css ?? ''} onChange={(event) => change({ ...content, css: event.target.value || undefined })} />}
                    </Field>
                    <p className="text-sm text-muted-foreground">{t('siteEditor.selectHelp')}</p>
                  </section>
                )
              )}
              {!block && Object.keys(patterns).length > 0 && (
                <section className="site-editor-section">
                  <h2>{t('siteEditor.insertPattern')}</h2>
                  <p className="text-sm text-muted-foreground">{t('siteEditor.insertPatternHelp')}</p>
                  <div className="site-editor-patterns">
                    {Object.entries(patterns).map(([key, pattern]) => (
                      <Button
                        key={key}
                        type="button"
                        size="sm"
                        variant="outline"
                        disabled={busy || entry.target === `pattern:${key}`}
                        onClick={() => {
                          const copies = pattern.blocks.map(duplicateBlock)
                          changeBlocks([...content.blocks, ...copies])
                          if (copies[0]) setSelected(copies[0].id)
                        }}
                      >
                        <Plus aria-hidden />
                        {patternNames[key] ?? pattern.name ?? key}
                      </Button>
                    ))}
                  </div>
                </section>
              )}
            </>
          )}
        </BlockScopeContext.Provider>
      </aside>
      <ConfirmDialog
        open={resetting}
        onOpenChange={setResetting}
        title={entry.fallback ? t('siteEditor.resetTitle') : t('siteEditor.deleteTitle')}
        description={entry.fallback ? t('siteEditor.resetDescription') : t('siteEditor.deleteDescription')}
        confirmLabel={entry.fallback ? t('siteEditor.reset') : t('siteEditor.delete')}
        variant="destructive"
        onConfirm={reset}
      />
    </>
  )
}
