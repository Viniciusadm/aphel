import type { Editor } from '@tiptap/react'
import { Bold, Code, Heading2, Heading3, Italic, List, ListOrdered, Quote, Redo2, Undo2, type LucideIcon } from 'lucide-react'

export interface RichAction {
  key: 'bold' | 'italic' | 'heading2' | 'heading3' | 'bulletList' | 'orderedList' | 'blockquote' | 'code' | 'undo' | 'redo'
  icon: LucideIcon
  active?: (editor: Editor) => boolean
  disabled?: (editor: Editor) => boolean
  run: (editor: Editor) => void
}

export const RICH_ACTIONS: RichAction[] = [
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

export function normalizeLink(raw: string): string | null {
  const value = raw.trim()
  if (!value) return null
  if (SAFE_LINK.test(value)) return value
  if (/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value)) return `mailto:${value}`
  return `https://${value}`
}


