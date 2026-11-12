import { EditorContent, useEditor, type Editor } from '@tiptap/react'
import { Placeholder } from '@tiptap/extensions'
import StarterKit from '@tiptap/starter-kit'
import { Eye, EyeOff, Maximize2, Minimize2, Plus } from 'lucide-react'
import { useCallback, useEffect, useMemo, useRef, useState, type ComponentType } from 'react'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { toast } from '@/components/ui/toast-store'
import { errorMessage } from '@/components/feedback'
import { uploadFile } from '@/features/files'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { cn } from '@/lib/utils'

import { BlockForm } from '../../blocks/BlockForm'
import { BLOCK_ICONS } from '../../blocks/blockIcons'
import { IMAGE_TYPES, MAX_BLOCKS, blockProblems, newBlock, type Block, type BlockType } from '../../blocks/blocks'
import { RichToolbar } from '../../blocks/richToolbar'
import type { DocumentBodyProps } from '../document'
import { useEditorialTranslation } from '../translation'
import { BlockCard } from './BlockCard'
import { blocksToHtml, docToBlocks, wordCount } from './flowDoc'

const EMIT_DELAY_MS = 250
const WORDS_PER_MINUTE = 200

type CardType = Extract<BlockType, 'image' | 'gallery' | 'video' | 'callout' | 'embed' | 'faq' | 'buttons'>
const CARD_TYPES: CardType[] = ['image', 'gallery', 'video', 'callout', 'embed', 'faq', 'buttons']

interface MenuItem {
  key: string
  label: string
  icon: ComponentType<{ 'aria-hidden'?: boolean; size?: number }>
  run: (editor: Editor) => void
  card?: CardType
}

interface Pending {
  block: Block
  position: number | null
}

function textItems(t: (key: string) => string): MenuItem[] {
  return [
    { key: 'heading2', label: t('flow.heading2'), icon: BLOCK_ICONS.hero, run: (editor) => editor.chain().focus().toggleHeading({ level: 2 }).run() },
    { key: 'heading3', label: t('flow.heading3'), icon: BLOCK_ICONS.hero, run: (editor) => editor.chain().focus().toggleHeading({ level: 3 }).run() },
    { key: 'bulletList', label: t('flow.bulletList'), icon: BLOCK_ICONS.faq, run: (editor) => editor.chain().focus().toggleBulletList().run() },
    { key: 'orderedList', label: t('flow.orderedList'), icon: BLOCK_ICONS.faq, run: (editor) => editor.chain().focus().toggleOrderedList().run() },
    { key: 'quote', label: t('flow.quote'), icon: BLOCK_ICONS.quote, run: (editor) => editor.chain().focus().toggleBlockquote().run() },
    { key: 'code', label: t('flow.code'), icon: BLOCK_ICONS.code, run: (editor) => editor.chain().focus().toggleCodeBlock().run() },
    { key: 'divider', label: t('flow.divider'), icon: BLOCK_ICONS.spacer, run: (editor) => editor.chain().focus().setHorizontalRule().run() },
  ]
}

function slashQuery(editor: Editor): { from: number; query: string } | null {
  const { $from, empty } = editor.state.selection
  if (!empty || $from.parent.type.name !== 'paragraph') return null
  const text = $from.parent.textContent
  const match = /^\/([\p{L}\d]*)$/u.exec(text)
  if (!match) return null
  return { from: $from.start(), query: match[1].toLowerCase() }
}

export function FlowBody({ record, recordType, title, blocks, disabled, translating, onTitleChange, onBlocksChange, focus, onFocusChange, preview, onPreviewChange }: DocumentBodyProps) {
  const t = useEditorialTranslation()
  const blockLabels = useModuleTranslation('site-blocks')
  const [pending, setPending] = useState<Pending | null>(null)
  const [menu, setMenu] = useState<{ query: string; top: number; left: number } | null>(null)
  const [active, setActive] = useState(0)
  const [words, setWords] = useState(0)
  const emitted = useRef(JSON.stringify(blocks))
  const timer = useRef<number | null>(null)
  const container = useRef<HTMLDivElement>(null)
  const titleRef = useRef<HTMLTextAreaElement>(null)
  const target = useMemo(() => ({ recordType, recordId: record.id }), [recordType, record.id])
  const disabledRef = useRef(disabled)
  const blocksRef = useRef(blocks)
  useEffect(() => {
    disabledRef.current = disabled
    blocksRef.current = blocks
  }, [disabled, blocks])

  const items = useMemo<MenuItem[]>(
    () => [
      ...textItems(t),
      ...CARD_TYPES.map<MenuItem>((type) => ({ key: type, label: blockLabels(`types.${type}`), icon: BLOCK_ICONS[type], card: type, run: () => undefined })),
    ],
    [t, blockLabels],
  )
  const visible = menu ? items.filter((item) => item.label.toLowerCase().includes(menu.query) || item.key.toLowerCase().includes(menu.query)) : []

  const emit = useCallback(
    (editor: Editor) => {
      if (timer.current !== null) window.clearTimeout(timer.current)
      timer.current = window.setTimeout(() => {
        timer.current = null
        const next = docToBlocks(editor.getJSON())
        const serialized = JSON.stringify(next)
        if (serialized === emitted.current) return
        emitted.current = serialized
        onBlocksChange(next)
      }, EMIT_DELAY_MS)
    },
    [onBlocksChange],
  )

  const cardExtension = useMemo(
    () =>
      BlockCard.configure({
        onEdit: (position, block) => setPending({ block, position }),
        editable: () => !disabledRef.current,
      }),
    [],
  )

  const editor = useEditor({
    extensions: [
      StarterKit.configure({ underline: false, strike: false, heading: { levels: [2, 3] }, link: { openOnClick: false, autolink: true, defaultProtocol: 'https' } }),
      Placeholder.configure({ placeholder: t('flow.placeholder') }),
      cardExtension,
    ],
    content: blocksToHtml(blocks),
    editable: !disabled,
    editorProps: {
      attributes: { role: 'textbox', 'aria-multiline': 'true', 'aria-label': t('flow.bodyLabel'), class: 'flow-prose' },
      handleKeyDown: (_view, event) => {
        if (menu === null || visible.length === 0) return false
        if (event.key === 'ArrowDown') {
          setActive((value) => (value + 1) % visible.length)
          return true
        }
        if (event.key === 'ArrowUp') {
          setActive((value) => (value - 1 + visible.length) % visible.length)
          return true
        }
        if (event.key === 'Enter' || event.key === 'Tab') {
          choose(visible[Math.min(active, visible.length - 1)])
          return true
        }
        if (event.key === 'Escape') {
          setMenu(null)
          return true
        }
        return false
      },
      handlePaste: (_view, event) => {
        const file = Array.from(event.clipboardData?.files ?? []).find((candidate) => IMAGE_TYPES.includes(candidate.type))
        if (!file) return false
        void insertFile(file)
        return true
      },
      handleDrop: (_view, event) => {
        const file = Array.from(event.dataTransfer?.files ?? []).find((candidate) => IMAGE_TYPES.includes(candidate.type))
        if (!file) return false
        event.preventDefault()
        void insertFile(file)
        return true
      },
    },
    onUpdate: ({ editor: current }) => {
      emit(current)
      setWords(wordCount(current.getText()))
      updateMenu(current)
    },
    onSelectionUpdate: ({ editor: current }) => updateMenu(current),
  })

  function updateMenu(current: Editor) {
    const found = slashQuery(current)
    if (!found || !container.current) {
      setMenu(null)
      return
    }
    const coords = current.view.coordsAtPos(current.state.selection.from)
    const box = container.current.getBoundingClientRect()
    setMenu({ query: found.query, top: coords.bottom - box.top + 6, left: coords.left - box.left })
    setActive(0)
  }

  function removeSlash(current: Editor) {
    const found = slashQuery(current)
    if (!found) return
    current.chain().focus().deleteRange({ from: found.from, to: current.state.selection.from }).run()
  }

  function choose(item: MenuItem) {
    if (!editor) return
    removeSlash(editor)
    setMenu(null)
    if (item.card) openNew(item.card)
    else item.run(editor)
  }

  function openNew(type: CardType) {
    if (blocksRef.current.length >= MAX_BLOCKS) {
      toast({ title: t('editor.tooMany', { count: MAX_BLOCKS }), variant: 'destructive' })
      return
    }
    setPending({ block: newBlock(type), position: null })
  }

  async function insertFile(file: File) {
    if (!editor) return
    try {
      const uploaded = await uploadFile({ file, target })
      setPending({ block: { ...(newBlock('image') as Extract<Block, { type: 'image' }>), image: uploaded.file.id }, position: null })
    } catch (failure) {
      toast({ title: errorMessage(failure), variant: 'destructive' })
    }
  }

  function apply() {
    if (!editor || !pending) return
    if (blockProblems(pending.block).length > 0) {
      toast({ title: t('flow.blockIncomplete'), variant: 'destructive' })
      return
    }
    const attrs = { block: JSON.stringify(pending.block) }
    if (pending.position === null) editor.chain().focus().insertContent({ type: 'blockCard', attrs }).run()
    else editor.view.dispatch(editor.state.tr.setNodeMarkup(pending.position, undefined, attrs))
    setPending(null)
  }

  useEffect(() => {
    if (!editor) return
    editor.setEditable(!disabled)
  }, [editor, disabled])

  useEffect(() => {
    if (!editor) return
    const incoming = JSON.stringify(blocks)
    if (incoming === emitted.current) return
    emitted.current = incoming
    editor.commands.setContent(blocksToHtml(blocks), { emitUpdate: false })
    setWords(wordCount(editor.getText()))
  }, [editor, blocks])

  useEffect(() => {
    if (editor) setWords(wordCount(editor.getText()))
  }, [editor])

  useEffect(() => {
    const timerId = timer
    return () => {
      if (timerId.current !== null) window.clearTimeout(timerId.current)
    }
  }, [])

  useEffect(() => {
    if (!focus) return
    const leave = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onFocusChange(false)
    }
    window.addEventListener('keydown', leave)
    return () => window.removeEventListener('keydown', leave)
  }, [focus, onFocusChange])

  useEffect(() => {
    const field = titleRef.current
    if (!field) return
    field.style.height = 'auto'
    field.style.height = `${field.scrollHeight}px`
  }, [title])

  const minutes = Math.max(1, Math.ceil(words / WORDS_PER_MINUTE))
  const pendingBlock = pending?.block ?? null

  return (
    <div className="flow" data-focus={focus || undefined}>
      <div className="flow-bar">
        <RichToolbar editor={editor} disabled={disabled}>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button type="button" variant="ghost" size="sm" disabled={disabled} className="ml-1">
                <Plus aria-hidden />
                {t('flow.insert')}
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start">
              {items.map((item) => {
                const Icon = item.icon
                return (
                  <DropdownMenuItem key={item.key} onSelect={() => (item.card ? openNew(item.card) : editor && item.run(editor))}>
                    <Icon aria-hidden size={16} />
                    {item.label}
                  </DropdownMenuItem>
                )
              })}
            </DropdownMenuContent>
          </DropdownMenu>
          <span className="flow-bar-spacer" />
          <Button type="button" variant="ghost" size="sm" aria-pressed={preview} onClick={() => onPreviewChange(!preview)}>
            {preview ? <EyeOff aria-hidden /> : <Eye aria-hidden />}
            {preview ? t('flow.hidePreview') : t('flow.showPreview')}
          </Button>
          <Button type="button" variant="ghost" size="sm" aria-pressed={focus} onClick={() => onFocusChange(!focus)}>
            {focus ? <Minimize2 aria-hidden /> : <Maximize2 aria-hidden />}
            {focus ? t('flow.exitFocus') : t('flow.focus')}
          </Button>
        </RichToolbar>
      </div>
      <div className="flow-page" ref={container}>
        <textarea
          ref={titleRef}
          className="flow-title"
          rows={1}
          value={title}
          maxLength={160}
          disabled={disabled}
          placeholder={translating ? record.draft_title : t('flow.titlePlaceholder')}
          aria-label={t('editor.titleLabel')}
          onChange={(event) => onTitleChange(event.target.value.replace(/\n/g, ' '))}
          onKeyDown={(event) => {
            if (event.key === 'Enter') {
              event.preventDefault()
              editor?.commands.focus('start')
            }
          }}
        />
        <EditorContent editor={editor} />
        {menu && visible.length > 0 && (
          <ul className="flow-menu" role="listbox" style={{ top: menu.top, left: menu.left }} aria-label={t('flow.insert')}>
            {visible.map((item, index) => {
              const Icon = item.icon
              return (
                <li key={item.key} role="option" aria-selected={index === Math.min(active, visible.length - 1)}>
                  <button type="button" className={cn('flow-menu-item', index === Math.min(active, visible.length - 1) && 'flow-menu-active')} onMouseDown={(event) => event.preventDefault()} onClick={() => choose(item)}>
                    <Icon aria-hidden size={16} />
                    {item.label}
                  </button>
                </li>
              )
            })}
          </ul>
        )}
      </div>
      <p className="flow-stats" role="status">
        {t('flow.words', { count: words })} · {t('flow.readingTime', { count: minutes })}
      </p>
      <Dialog open={pendingBlock !== null} onOpenChange={(open) => !open && setPending(null)}>
        <DialogContent size="2xl">
          <DialogHeader>
            <DialogTitle>{pendingBlock ? blockLabels(`types.${pendingBlock.type}`) : ''}</DialogTitle>
            <DialogDescription>{t('flow.blockDescription')}</DialogDescription>
          </DialogHeader>
          {pendingBlock && <BlockForm block={pendingBlock} record={target} disabled={disabled} onChange={(block) => setPending((value) => (value ? { ...value, block } : value))} />}
          <div className="flow-dialog-actions">
            <Button type="button" variant="ghost" onClick={() => setPending(null)}>
              {t('flow.cancel')}
            </Button>
            <Button type="button" onClick={apply}>
              {pending?.position === null ? t('flow.insertBlock') : t('flow.applyBlock')}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  )
}
