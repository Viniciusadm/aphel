import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'

export type CanvasDevice = 'desktop' | 'tablet' | 'mobile'

const WIDTHS: Record<CanvasDevice, number | null> = { desktop: null, tablet: 820, mobile: 390 }
const EDITOR_STYLE = `.blk-slot{cursor:pointer;outline-offset:2px;transition:outline-color .1s}
.blk-slot[data-editable]:hover{outline:1px dashed #6d7cff}
.blk-slot[data-selected]{outline:2px solid #4f46e5 !important}
a,button,input,textarea,select,summary{pointer-events:none}`

interface DesignCanvasProps {
  html: string
  device: CanvasDevice
  editable: Set<string>
  selected: string | null
  onSelect: (id: string | null) => void
  loading: boolean
}

export function DesignCanvas({ html, device, editable, selected, onSelect, loading }: DesignCanvasProps) {
  const { t } = useTranslation('site')
  const frame = useRef<HTMLIFrameElement>(null)
  const select = useRef(onSelect)
  const editableRef = useRef(editable)
  useEffect(() => {
    select.current = onSelect
    editableRef.current = editable
  })

  function decorate() {
    const document = frame.current?.contentDocument
    if (!document) return
    if (!document.getElementById('aphel-editor-style')) {
      const style = document.createElement('style')
      style.id = 'aphel-editor-style'
      style.textContent = EDITOR_STYLE
      document.head?.append(style)
    }
    for (const slot of document.querySelectorAll<HTMLElement>('.blk-slot')) {
      if (editableRef.current.has(slot.id)) slot.dataset.editable = ''
      else delete slot.dataset.editable
      if (slot.id === selected) slot.dataset.selected = ''
      else delete slot.dataset.selected
    }
  }

  useEffect(() => {
    decorate()
  })

  useEffect(() => {
    if (!selected) return
    const slot = frame.current?.contentDocument?.getElementById(selected)
    slot?.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
  }, [selected])

  function attach() {
    const document = frame.current?.contentDocument
    if (!document) return
    decorate()
    document.addEventListener(
      'click',
      (event) => {
        event.preventDefault()
        event.stopPropagation()
        let slot = (event.target as Element | null)?.closest?.('.blk-slot') as HTMLElement | null
        while (slot && !editableRef.current.has(slot.id)) slot = slot.parentElement?.closest('.blk-slot') as HTMLElement | null
        select.current(slot?.id ?? null)
      },
      true,
    )
  }

  const width = WIDTHS[device]
  return (
    <div className="site-canvas" data-loading={loading || undefined}>
      <iframe
        ref={frame}
        title={t('siteEditor.canvas')}
        sandbox="allow-same-origin"
        srcDoc={html}
        onLoad={attach}
        style={{ width: width ? `${width}px` : '100%' }}
      />
    </div>
  )
}
