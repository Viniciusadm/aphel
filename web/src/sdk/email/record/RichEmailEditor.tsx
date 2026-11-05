import { Placeholder } from '@tiptap/extensions'
import { EditorContent, useEditor, useEditorState, type Editor } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import {
  Bold,
  Italic,
  Link2,
  Link2Off,
  List,
  ListOrdered,
  Quote,
  Redo2,
  Strikethrough,
  Underline,
  Undo2,
  type LucideIcon,
} from 'lucide-react'
import { useImperativeHandle, useState, type Ref } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

export interface RichEmailEditorHandle {
  insertText: (text: string) => void
}

export interface RichEmailEditorProps {
  id: string
  label: string
  value: string
  placeholder?: string
  invalid?: boolean
  onChange: (html: string) => void
  onFocus?: () => void
  ref?: Ref<RichEmailEditorHandle>
}

const SAFE_LINK = /^(https?:|mailto:|tel:)/i

function normalizeLink(raw: string): string | null {
  const value = raw.trim()
  if (!value) return null
  if (SAFE_LINK.test(value)) return value
  if (/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)) return `mailto:${value}`
  return `https://${value}`
}

interface ToolbarAction {
  key: string
  icon: LucideIcon
  active?: (editor: Editor) => boolean
  disabled?: (editor: Editor) => boolean
  run: (editor: Editor) => void
}

const ACTIONS: ToolbarAction[] = [
  {
    key: 'bold',
    icon: Bold,
    active: (editor) => editor.isActive('bold'),
    run: (editor) => editor.chain().focus().toggleBold().run(),
  },
  {
    key: 'italic',
    icon: Italic,
    active: (editor) => editor.isActive('italic'),
    run: (editor) => editor.chain().focus().toggleItalic().run(),
  },
  {
    key: 'underline',
    icon: Underline,
    active: (editor) => editor.isActive('underline'),
    run: (editor) => editor.chain().focus().toggleUnderline().run(),
  },
  {
    key: 'strike',
    icon: Strikethrough,
    active: (editor) => editor.isActive('strike'),
    run: (editor) => editor.chain().focus().toggleStrike().run(),
  },
  {
    key: 'bulletList',
    icon: List,
    active: (editor) => editor.isActive('bulletList'),
    run: (editor) => editor.chain().focus().toggleBulletList().run(),
  },
  {
    key: 'orderedList',
    icon: ListOrdered,
    active: (editor) => editor.isActive('orderedList'),
    run: (editor) => editor.chain().focus().toggleOrderedList().run(),
  },
  {
    key: 'blockquote',
    icon: Quote,
    active: (editor) => editor.isActive('blockquote'),
    run: (editor) => editor.chain().focus().toggleBlockquote().run(),
  },
  {
    key: 'undo',
    icon: Undo2,
    disabled: (editor) => !editor.can().undo(),
    run: (editor) => editor.chain().focus().undo().run(),
  },
  {
    key: 'redo',
    icon: Redo2,
    disabled: (editor) => !editor.can().redo(),
    run: (editor) => editor.chain().focus().redo().run(),
  },
]

export default function RichEmailEditor({
  id,
  label,
  value,
  placeholder,
  invalid,
  onChange,
  onFocus,
  ref,
}: RichEmailEditorProps) {
  const { t } = useTranslation('email', { keyPrefix: 'composer.editor' })
  const [linkDraft, setLinkDraft] = useState<string | null>(null)
  const editor = useEditor({
    extensions: [
      StarterKit.configure({
        heading: false,
        code: false,
        codeBlock: false,
        horizontalRule: false,
        link: { openOnClick: false, autolink: true, defaultProtocol: 'https' },
      }),
      Placeholder.configure({ placeholder: placeholder ?? '' }),
    ],
    content: value,
    editorProps: {
      attributes: {
        id,
        role: 'textbox',
        'aria-multiline': 'true',
        ...(placeholder ? { 'aria-placeholder': placeholder } : {}),
        'aria-label': label,
        ...(invalid ? { 'aria-invalid': 'true' } : {}),
        class:
          'min-h-56 px-3 py-2 text-sm outline-none [&_.is-editor-empty:first-child]:before:pointer-events-none [&_.is-editor-empty:first-child]:before:float-left [&_.is-editor-empty:first-child]:before:h-0 [&_.is-editor-empty:first-child]:before:text-muted-foreground [&_.is-editor-empty:first-child]:before:content-[attr(data-placeholder)] [&_a]:text-primary [&_a]:underline [&_blockquote]:border-l-2 [&_blockquote]:pl-3 [&_blockquote]:text-muted-foreground [&_ol]:list-decimal [&_ol]:pl-6 [&_p]:my-1 [&_ul]:list-disc [&_ul]:pl-6',
      },
    },
    onUpdate: ({ editor: current }) => onChange(current.isEmpty ? '' : current.getHTML()),
    onFocus: () => onFocus?.(),
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

  useImperativeHandle(
    ref,
    () => ({
      insertText: (text: string) => {
        editor?.chain().focus().insertContent(text).run()
      },
    }),
    [editor],
  )

  const openLink = () => {
    if (!editor) return
    const current = editor.getAttributes('link').href
    setLinkDraft(typeof current === 'string' ? current : '')
  }

  const applyLink = () => {
    if (!editor || linkDraft === null) return
    const href = normalizeLink(linkDraft)
    const chain = editor.chain().focus().extendMarkRange('link')
    if (href) chain.setLink({ href }).run()
    else chain.unsetLink().run()
    setLinkDraft(null)
  }

  return (
    <div
      className={cn(
        'relative rounded-md border border-input bg-background shadow-xs focus-within:ring-2 focus-within:ring-ring',
        invalid && 'border-destructive',
      )}
    >
      <div role="toolbar" aria-label={t('toolbar')} className="flex flex-wrap items-center gap-0.5 border-b p-1">
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
              aria-label={t(action.key as 'bold')}
              title={t(action.key as 'bold')}
              aria-pressed={action.active ? pressed : undefined}
              disabled={!editor || (state?.disabled[action.key] ?? false)}
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
          aria-label={t('link')}
          title={t('link')}
          aria-pressed={state?.link ?? false}
          disabled={!editor}
          onClick={openLink}
        >
          <Link2 aria-hidden="true" />
        </Button>
        {state?.link && (
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="size-7"
            aria-label={t('unlink')}
            title={t('unlink')}
            onClick={() => editor?.chain().focus().extendMarkRange('link').unsetLink().run()}
          >
            <Link2Off aria-hidden="true" />
          </Button>
        )}
      </div>
      {linkDraft !== null && (
        <div className="flex flex-wrap items-center gap-2 border-b p-2">
          <Input
            autoFocus
            type="url"
            inputMode="url"
            aria-label={t('linkUrl')}
            placeholder="https://"
            value={linkDraft}
            className="h-8 min-w-0 flex-1"
            onChange={(event) => setLinkDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') {
                event.preventDefault()
                applyLink()
              }
              if (event.key === 'Escape') {
                event.preventDefault()
                event.stopPropagation()
                setLinkDraft(null)
              }
            }}
          />
          <Button type="button" size="sm" onClick={applyLink}>
            {t('applyLink')}
          </Button>
          <Button type="button" size="sm" variant="ghost" onClick={() => setLinkDraft(null)}>
            {t('cancelLink')}
          </Button>
        </div>
      )}
      <EditorContent editor={editor} />
    </div>
  )
}
