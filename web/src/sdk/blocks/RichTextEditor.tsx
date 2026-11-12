import { EditorContent, useEditor, useEditorState, type Editor } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import { Bold, Code, Heading2, Heading3, Italic, Link2, Link2Off, List, ListOrdered, Quote, Redo2, Undo2, type LucideIcon } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { cn } from '@/lib/utils'
import { docToMarkdown, markdownToHtml } from './richtext'

interface Action {
  key: 'bold' | 'italic' | 'heading2' | 'heading3' | 'bulletList' | 'orderedList' | 'blockquote' | 'code' | 'undo' | 'redo'
  icon: LucideIcon
  active?: (editor: Editor) => boolean
  disabled?: (editor: Editor) => boolean
  run: (editor: Editor) => void
}

const ACTIONS: Action[] = [
  { key: 'bold', icon: Bold, active: (editor) => editor.isActive('bold'), run: (editor) => editor.chain().focus().toggleBold().run() },
  { key: 'italic', icon: Italic, active: (editor) => editor.isActive('italic'), run: (editor) => editor.chain().focus().toggleItalic().run() },
  { key: 'heading2', icon: Heading2, active: (editor) => editor.isActive('heading', { level: 2 }), run: (editor) => editor.chain().focus().toggleHeading({ level: 2 }).run() },
  { key: 'heading3', icon: Heading3, active: (editor) => editor.isActive('heading', { level: 3 }), run: (editor) => editor.chain().focus().toggleHeading({ level: 3 }).run() },
  { key: 'bulletList', icon: List, active: (editor) => editor.isActive('bulletList'), run: (editor) => editor.chain().focus().toggleBulletList().run() },
  { key: 'orderedList', icon: ListOrdered, active: (editor) => editor.isActive('orderedList'), run: (editor) => editor.chain().focus().toggleOrderedList().run() },
  { key: 'blockquote', icon: Quote, active: (editor) => editor.isActive('blockquote'), run: (editor) => editor.chain().focus().toggleBlockquote().run() },
  { key: 'code', icon: Code, active: (editor) => editor.isActive('code'), run: (editor) => editor.chain().focus().toggleCode().run() },
  { key: 'undo', icon: Undo2, disabled: (editor) => !editor.can().undo(), run: (editor) => editor.chain().focus().undo().run() },
  { key: 'redo', icon: Redo2, disabled: (editor) => !editor.can().redo(), run: (editor) => editor.chain().focus().redo().run() },
]

const SAFE_LINK = /^(https?:|mailto:|tel:|\/)/i

function normalizeLink(raw: string): string | null {
  const value = raw.trim()
  if (!value) return null
  if (SAFE_LINK.test(value)) return value
  if (/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)) return `mailto:${value}`
  return `https://${value}`
}

export interface RichTextEditorProps {
  id?: string
  label: string
  value: string
  disabled?: boolean
  invalid?: boolean
  describedBy?: string
  onChange: (markdown: string) => void
}

export default function RichTextEditor({ id, label, value, disabled, invalid, describedBy, onChange }: RichTextEditorProps) {
  const t = useModuleTranslation('site-blocks')
  const [linkDraft, setLinkDraft] = useState<string | null>(null)
  const emitted = useRef(value)
  const editor = useEditor({
    extensions: [
      StarterKit.configure({
        underline: false,
        strike: false,
        horizontalRule: false,
        codeBlock: false,
        link: { openOnClick: false, autolink: true, defaultProtocol: 'https' },
      }),
    ],
    content: markdownToHtml(value),
    editable: !disabled,
    editorProps: {
      attributes: {
        ...(id ? { id } : {}),
        role: 'textbox',
        'aria-multiline': 'true',
        'aria-label': label,
        ...(describedBy ? { 'aria-describedby': describedBy } : {}),
        ...(invalid ? { 'aria-invalid': 'true' } : {}),
        class:
          'min-h-40 px-3 py-2 text-sm outline-none [&_a]:text-primary [&_a]:underline [&_blockquote]:border-l-2 [&_blockquote]:pl-3 [&_blockquote]:text-muted-foreground [&_code]:rounded [&_code]:bg-muted [&_code]:px-1 [&_h2]:mt-3 [&_h2]:text-lg [&_h2]:font-semibold [&_h3]:mt-2 [&_h3]:font-semibold [&_ol]:list-decimal [&_ol]:pl-6 [&_p]:my-1 [&_ul]:list-disc [&_ul]:pl-6',
      },
    },
    onUpdate: ({ editor: current }) => {
      const markdown = current.isEmpty ? '' : docToMarkdown(current.getJSON())
      emitted.current = markdown
      onChange(markdown)
    },
  })

  const state = useEditorState({
    editor,
    selector: ({ editor: current }) =>
      current
        ? {
            active: Object.fromEntries(ACTIONS.map((action) => [action.key, action.active?.(current) ?? false])),
            disabled: Object.fromEntries(ACTIONS.map((action) => [action.key, action.disabled?.(current) ?? false])),
            link: current.isActive('link'),
          }
        : null,
  })

  useEffect(() => {
    if (!editor || value === emitted.current) return
    emitted.current = value
    editor.commands.setContent(markdownToHtml(value), { emitUpdate: false })
  }, [editor, value])

  const applyLink = () => {
    if (!editor || linkDraft === null) return
    const href = normalizeLink(linkDraft)
    const chain = editor.chain().focus().extendMarkRange('link')
    if (href) chain.setLink({ href }).run()
    else chain.unsetLink().run()
    setLinkDraft(null)
  }

  return (
    <div className={cn('rounded-md border border-input bg-background shadow-xs focus-within:ring-2 focus-within:ring-ring', invalid && 'border-destructive')}>
      <div role="toolbar" aria-label={t('rich.toolbar')} className="flex flex-wrap items-center gap-0.5 border-b p-1">
        {ACTIONS.map((action) => {
          const Icon = action.icon
          const pressed = state?.active[action.key] ?? false
          return (
            <Button
              key={action.key}
              type="button"
              variant="ghost"
              size="icon"
              className={cn('size-7', pressed && 'bg-accent text-accent-foreground')}
              aria-label={t(`rich.${action.key}`)}
              title={t(`rich.${action.key}`)}
              aria-pressed={action.active ? pressed : undefined}
              disabled={disabled || !editor || (state?.disabled[action.key] ?? false)}
              onClick={() => editor && action.run(editor)}
            >
              <Icon aria-hidden="true" />
            </Button>
          )
        })}
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className={cn('size-7', state?.link && 'bg-accent text-accent-foreground')}
          aria-label={t('rich.link')}
          title={t('rich.link')}
          aria-pressed={state?.link ?? false}
          disabled={disabled || !editor}
          onClick={() => editor && setLinkDraft(String(editor.getAttributes('link').href ?? ''))}
        >
          <Link2 aria-hidden="true" />
        </Button>
        {state?.link && (
          <Button type="button" variant="ghost" size="icon" className="size-7" aria-label={t('rich.unlink')} title={t('rich.unlink')} onClick={() => editor?.chain().focus().extendMarkRange('link').unsetLink().run()}>
            <Link2Off aria-hidden="true" />
          </Button>
        )}
      </div>
      {linkDraft !== null && (
        <div className="flex flex-wrap items-center gap-2 border-b p-2">
          <Input
            autoFocus
            inputMode="url"
            aria-label={t('rich.linkUrl')}
            placeholder="https://"
            value={linkDraft}
            className="h-8 min-w-0 flex-1"
            onChange={(event) => setLinkDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') {
                event.preventDefault()
                applyLink()
              } else if (event.key === 'Escape') {
                event.preventDefault()
                setLinkDraft(null)
                editor?.commands.focus()
              }
            }}
          />
          <Button type="button" size="sm" onClick={applyLink}>
            {t('rich.apply')}
          </Button>
          <Button type="button" size="sm" variant="ghost" onClick={() => setLinkDraft(null)}>
            {t('rich.cancel')}
          </Button>
        </div>
      )}
      <EditorContent editor={editor} />
    </div>
  )
}
