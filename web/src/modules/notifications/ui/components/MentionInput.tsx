import {
  type ChangeEvent,
  type ComponentProps,
  type KeyboardEvent,
  type Ref,
  type SyntheticEvent,
  useId,
  useMemo,
  useRef,
  useState,
} from 'react'
import { useTranslation } from 'react-i18next'
import { Textarea } from '@/components/ui/textarea'
import { useOrganizationUsers } from '@/sdk'
import { cn } from '@/lib/utils'
import { type MentionQuery, findMentionQuery, insertMention } from '@/modules/notifications/ui/lib/mentions'

export const MAX_MENTION_SUGGESTIONS = 6

export type MentionInputProps = Omit<ComponentProps<'textarea'>, 'value' | 'ref'> & {
  value: string
  onValueChange?: (value: string) => void
  ref?: Ref<HTMLTextAreaElement>
}

function normalize(value: string): string {
  return value
    .normalize('NFD')
    .replace(/\p{Diacritic}/gu, '')
    .toLowerCase()
}

function changeEvent(element: HTMLTextAreaElement): ChangeEvent<HTMLTextAreaElement> {
  const nativeEvent = new Event('input', { bubbles: true })
  return {
    nativeEvent,
    target: element,
    currentTarget: element,
    type: 'change',
    bubbles: true,
    cancelable: false,
    defaultPrevented: false,
    eventPhase: nativeEvent.eventPhase,
    isTrusted: false,
    timeStamp: nativeEvent.timeStamp,
    preventDefault: () => {},
    isDefaultPrevented: () => false,
    stopPropagation: () => {},
    isPropagationStopped: () => false,
    persist: () => {},
  }
}

export function MentionInput({
  value,
  onValueChange,
  onChange,
  onKeyDown,
  onBlur,
  className,
  ref,
  ...props
}: MentionInputProps) {
  const { t } = useTranslation('notifications')
  const listId = useId()
  const textarea = useRef<HTMLTextAreaElement>(null)
  const attach = (element: HTMLTextAreaElement | null) => {
    textarea.current = element
    if (typeof ref === 'function') ref(element)
    else if (ref) ref.current = element
  }
  const [mention, setMention] = useState<MentionQuery | null>(null)
  const [active, setActive] = useState(0)
  const users = useOrganizationUsers()

  const suggestions = useMemo(() => {
    if (!mention) return []
    const query = normalize(mention.query)
    return (users.data ?? [])
      .filter((user) => normalize(user.display_name).includes(query))
      .slice(0, MAX_MENTION_SUGGESTIONS)
  }, [mention, users.data])

  const open = mention !== null && !props.disabled && !props.readOnly

  const detect = (element: HTMLTextAreaElement) => {
    const caret = element.selectionStart ?? element.value.length
    const next = element.selectionStart === element.selectionEnd ? findMentionQuery(element.value, caret) : null
    setMention(next)
    if (next?.query !== mention?.query) setActive(0)
  }

  const select = (index: number) => {
    const element = textarea.current
    const user = suggestions[index]
    if (!element || !mention || !user) return
    const result = insertMention(element.value, mention, user.display_name, user.id)
    setMention(null)
    element.value = result.value
    onChange?.(changeEvent(element))
    onValueChange?.(result.value)
    element.focus()
    element.setSelectionRange(result.caret, result.caret)
  }

  const handleKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (open && suggestions.length > 0 && !event.metaKey && !event.ctrlKey) {
      if (event.key === 'ArrowDown') {
        event.preventDefault()
        setActive((current) => (current + 1) % suggestions.length)
        return
      }
      if (event.key === 'ArrowUp') {
        event.preventDefault()
        setActive((current) => (current - 1 + suggestions.length) % suggestions.length)
        return
      }
      if (event.key === 'Enter' || event.key === 'Tab') {
        event.preventDefault()
        select(active)
        return
      }
    }
    if (open && event.key === 'Escape') {
      event.preventDefault()
      setMention(null)
      return
    }
    onKeyDown?.(event)
  }

  const handleSelect = (event: SyntheticEvent<HTMLTextAreaElement>) => detect(event.currentTarget)

  const optionId = (index: number) => `${listId}-option-${index}`

  return (
    <div className="relative">
      <Textarea
        {...props}
        ref={attach}
        value={value}
        className={className}
        aria-autocomplete="list"
        aria-controls={open ? listId : undefined}
        aria-activedescendant={open && suggestions.length > 0 ? optionId(active) : undefined}
        onChange={(event) => {
          onChange?.(event)
          onValueChange?.(event.target.value)
          detect(event.currentTarget)
        }}
        onKeyDown={handleKeyDown}
        onKeyUp={(event) => {
          if (event.key === 'ArrowLeft' || event.key === 'ArrowRight' || event.key === 'Home' || event.key === 'End') {
            handleSelect(event)
          }
        }}
        onClick={handleSelect}
        onBlur={(event) => {
          setMention(null)
          onBlur?.(event)
        }}
      />
      {open && (
        <ul
          id={listId}
          role="listbox"
          aria-label={t('mention.listLabel')}
          className="absolute top-full left-0 z-50 mt-1 max-h-60 w-full max-w-xs overflow-y-auto rounded-md border bg-popover p-1 text-sm text-popover-foreground shadow-md"
        >
          {suggestions.length === 0 ? (
            <li role="presentation" className="px-2 py-1.5 text-muted-foreground">
              {users.isPending ? t('mention.loading') : t('mention.empty')}
            </li>
          ) : (
            suggestions.map((user, index) => (
              <li
                key={user.id}
                id={optionId(index)}
                role="option"
                aria-selected={index === active}
                className={cn(
                  'cursor-pointer truncate rounded-sm px-2 py-1.5',
                  index === active && 'bg-accent text-accent-foreground',
                )}
                onMouseDown={(event) => {
                  event.preventDefault()
                  select(index)
                }}
                onMouseEnter={() => setActive(index)}
              >
                {user.display_name}
              </li>
            ))
          )}
        </ul>
      )}
    </div>
  )
}
