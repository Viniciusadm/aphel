import { ImageOff } from 'lucide-react'
import { useCallback, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

const MIN_HEIGHT = 80
const MAX_HEIGHT = 4000
const REMOTE_IMAGE_PATTERN = /<img[^>]+src\s*=\s*["']?\s*(https?:)?\/\//i

function frameDocument(html: string, allowRemoteImages: boolean): string {
  const images = allowRemoteImages ? 'data: https: http:' : 'data:'
  const policy = `default-src 'none'; img-src ${images}; style-src 'unsafe-inline'; font-src data:`
  return `<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="${policy}"><base target="_blank"><style>html,body{margin:0;padding:0;background:#fff;color:#111;font-family:system-ui,-apple-system,"Segoe UI",sans-serif;font-size:14px;line-height:1.5;overflow-wrap:anywhere}body{padding:12px}img{max-width:100%;height:auto}table{max-width:100%}pre{white-space:pre-wrap}</style></head><body>${html}</body></html>`
}

export interface HtmlFrameProps {
  html: string
  title: string
  className?: string
}

export function HtmlFrame({ html, title, className }: HtmlFrameProps) {
  const { t } = useTranslation('email')
  const frame = useRef<HTMLIFrameElement>(null)
  const [height, setHeight] = useState(MIN_HEIGHT)
  const [remoteImages, setRemoteImages] = useState(false)
  const hasRemoteImages = useMemo(() => REMOTE_IMAGE_PATTERN.test(html), [html])
  const document = useMemo(() => frameDocument(html, remoteImages), [html, remoteImages])

  const measure = useCallback(() => {
    const body = frame.current?.contentDocument?.documentElement
    if (!body) return
    setHeight(Math.min(MAX_HEIGHT, Math.max(MIN_HEIGHT, body.scrollHeight + 2)))
  }, [])

  return (
    <div className={cn('grid gap-2', className)}>
      {hasRemoteImages && !remoteImages && (
        <div className="flex flex-wrap items-center justify-between gap-2 rounded-md border bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
          <span className="flex items-center gap-1.5">
            <ImageOff className="size-3.5" aria-hidden="true" />
            {t('reader.remoteImagesBlocked')}
          </span>
          <Button variant="outline" size="sm" onClick={() => setRemoteImages(true)}>
            {t('reader.showImages')}
          </Button>
        </div>
      )}
      <iframe
        ref={frame}
        title={title}
        sandbox="allow-same-origin allow-popups allow-popups-to-escape-sandbox"
        referrerPolicy="no-referrer"
        srcDoc={document}
        onLoad={measure}
        style={{ height }}
        className="w-full rounded-md border bg-white"
      />
    </div>
  )
}

export function TextBody({ text }: { text: string }) {
  return <div className="text-sm break-words whitespace-pre-wrap">{text}</div>
}
