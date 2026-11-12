import { useEditorState, type Editor } from '@tiptap/react'
import { Link2, Link2Off } from 'lucide-react'
import { useState, type ReactNode } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { cn } from '@/lib/utils'

import { RICH_ACTIONS, normalizeLink, type RichAction } from './richActions'

export function RichToolbar({ editor, disabled, children, only }: { editor: Editor | null; disabled?: boolean; children?: ReactNode; only?: RichAction['key'][] }) {
  const t = useModuleTranslation('site-blocks')
  const [linkDraft, setLinkDraft] = useState<string | null>(null)
  const actions = only ? RICH_ACTIONS.filter((action) => only.includes(action.key)) : RICH_ACTIONS
  const state = useEditorState({
    editor,
    selector: ({ editor: current }) =>
      current
        ? {
            active: Object.fromEntries(actions.map((action) => [action.key, action.active?.(current) ?? false])),
            disabled: Object.fromEntries(actions.map((action) => [action.key, action.disabled?.(current) ?? false])),
            link: current.isActive('link'),
          }
        : null,
  })

  const applyLink = () => {
    if (!editor || linkDraft === null) return
    const href = normalizeLink(linkDraft)
    const chain = editor.chain().focus().extendMarkRange('link')
    if (href) chain.setLink({ href }).run()
    else chain.unsetLink().run()
    setLinkDraft(null)
  }

  return (
    <>
      <div role="toolbar" aria-label={t('rich.toolbar')} className="flex flex-wrap items-center gap-0.5 border-b p-1">
        {actions.map((action) => {
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
        {children}
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
    </>
  )
}
