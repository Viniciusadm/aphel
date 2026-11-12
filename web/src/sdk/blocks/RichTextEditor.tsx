import { EditorContent, useEditor } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import { useEffect, useRef } from 'react'
import { cn } from '@/lib/utils'
import { RichToolbar } from './richToolbar'
import { docToMarkdown, markdownToHtml } from './richtext'

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

  useEffect(() => {
    if (!editor || value === emitted.current) return
    emitted.current = value
    editor.commands.setContent(markdownToHtml(value), { emitUpdate: false })
  }, [editor, value])

  return (
    <div className={cn('rounded-md border border-input bg-background shadow-xs focus-within:ring-2 focus-within:ring-ring', invalid && 'border-destructive')}>
      <RichToolbar editor={editor} disabled={disabled} />
      <EditorContent editor={editor} />
    </div>
  )
}
