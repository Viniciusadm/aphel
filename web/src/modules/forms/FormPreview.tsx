import { Monitor, Smartphone } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { SegmentedControl } from '@/components/ui/segmented-control'
import { Skeleton } from '@/components/ui/skeleton'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { FORMS_KEY, formsApi } from './api'
import { toRequestBody, type LeadFormValues } from './form'

type Device = 'desktop' | 'mobile'

const WIDTHS: Record<Device, number> = { desktop: 640, mobile: 390 }
const FRAME_HEIGHT = 720
const DEBOUNCE_MS = 400

interface FormPreviewProps {
  values: LeadFormValues
  publicId?: string
}

export function FormPreview({ values, publicId }: FormPreviewProps) {
  const t = useModuleTranslation(FORMS_KEY)
  const [device, setDevice] = useState<Device>('desktop')
  const [html, setHtml] = useState<string | null>(null)
  const [failed, setFailed] = useState(false)
  const [available, setAvailable] = useState(0)
  const holder = useRef<HTMLDivElement>(null)
  const request = useRef(0)
  const signature = JSON.stringify(values)

  useEffect(() => {
    const node = holder.current
    if (!node) return
    const observer = new ResizeObserver(() => setAvailable(node.clientWidth))
    observer.observe(node)
    return () => observer.disconnect()
  }, [])

  useEffect(() => {
    const current = ++request.current
    const timer = window.setTimeout(() => {
      void formsApi
        .preview({ ...toRequestBody(JSON.parse(signature) as LeadFormValues), public_id: publicId })
        .then((result) => {
          if (current !== request.current) return
          setHtml(result.html)
          setFailed(false)
        })
        .catch(() => {
          if (current === request.current) setFailed(true)
        })
    }, DEBOUNCE_MS)
    return () => window.clearTimeout(timer)
  }, [signature, publicId])

  const width = WIDTHS[device]
  const scale = available > 0 ? Math.min(1, available / width) : 1

  return (
    <div className="grid gap-2">
      <SegmentedControl
        aria-label={t('preview.device')}
        value={device}
        onValueChange={setDevice}
        options={[
          { value: 'desktop', label: <Monitor className="size-4" aria-hidden="true" />, ariaLabel: t('preview.desktop') },
          { value: 'mobile', label: <Smartphone className="size-4" aria-hidden="true" />, ariaLabel: t('preview.mobile') },
        ]}
      />
      <div ref={holder} className="overflow-hidden rounded-md border bg-muted/30" style={{ height: FRAME_HEIGHT * scale }}>
        {html === null ? (
          <Skeleton className="h-full w-full" />
        ) : (
          <iframe
            title={t('preview.label')}
            sandbox=""
            srcDoc={html}
            className="block border-0 bg-transparent"
            style={{ width, height: FRAME_HEIGHT, transform: `scale(${scale})`, transformOrigin: 'top left' }}
          />
        )}
      </div>
      {failed && <p className="text-xs text-destructive">{t('preview.failed')}</p>}
      <p className="text-xs text-muted-foreground">{t(publicId ? 'preview.hint' : 'preview.hintNew')}</p>
    </div>
  )
}
