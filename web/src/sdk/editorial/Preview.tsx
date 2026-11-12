import { useEditorialTranslation } from './translation'
import { Button, SegmentedControl, Spinner } from '../ui'
import { ExternalLink, Monitor, Smartphone } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'

import type { PreviewLink } from './document'

const DESKTOP_WIDTH = 1280
const MOBILE_WIDTH = 390

type Device = 'desktop' | 'mobile'

export interface LiveFrame {
  html: string
  anchor: string | null
}

interface PreviewProps {
  link: PreviewLink | null
  failed: boolean
  nonce: number
  anchor: string | null
  live: LiveFrame | null
  onLiveChange: (ready: boolean) => void
}

const READY_MESSAGE = 'aphel:preview-ready'
const HTML_MESSAGE = 'aphel:preview-html'
const READY_TIMEOUT_MS = 3000

function sameOrigin(url: string): boolean {
  try {
    return new URL(url, window.location.href).origin === window.location.origin
  } catch {
    return false
  }
}

function withNonce(url: string, nonce: number, anchor: string | null): string {
  const target = new URL(url, window.location.href)
  target.searchParams.set('_preview', String(nonce))
  target.hash = anchor ?? ''
  return target.toString()
}

export function Preview({ link, failed, nonce, anchor, live, onLiveChange }: PreviewProps) {
  const t = useEditorialTranslation()
  const [device, setDevice] = useState<Device>('desktop')
  const stage = useRef<HTMLDivElement>(null)
  const [size, setSize] = useState({ width: 0, height: 0 })
  const [loading, setLoading] = useState(true)
  const [primed, setPrimed] = useState<{ url: string; nonce: number } | null>(null)
  const frame = useRef<HTMLIFrameElement>(null)
  const [liveReady, setLiveReady] = useState(false)
  const liveReadyRef = useRef(false)
  liveReadyRef.current = liveReady
  const latestLive = useRef(live)
  latestLive.current = live
  const reportLive = useRef(onLiveChange)
  reportLive.current = onLiveChange

  useEffect(() => {
    const receive = (event: MessageEvent) => {
      if (event.origin !== window.location.origin || event.source !== frame.current?.contentWindow) return
      if ((event.data as { type?: string } | null)?.type !== READY_MESSAGE) return
      setLiveReady(true)
      reportLive.current(true)
      const current = latestLive.current
      if (current) frame.current?.contentWindow?.postMessage({ type: HTML_MESSAGE, ...current }, window.location.origin)
    }
    window.addEventListener('message', receive)
    return () => window.removeEventListener('message', receive)
  }, [])

  useEffect(() => {
    if (!liveReady || !live) return
    frame.current?.contentWindow?.postMessage({ type: HTML_MESSAGE, ...live }, window.location.origin)
  }, [live, liveReady])

  useEffect(() => {
    const element = stage.current
    if (!element) return
    const observer = new ResizeObserver(([entry]) => setSize({ width: entry.contentRect.width, height: entry.contentRect.height }))
    observer.observe(element)
    return () => observer.disconnect()
  }, [])

  useEffect(() => {
    setLoading(true)
  }, [nonce, link?.url])

  const virtualWidth = device === 'desktop' ? DESKTOP_WIDTH : MOBILE_WIDTH
  const scale = size.width > 0 ? Math.min(1, size.width / virtualWidth) : 1
  const embeddable = link ? sameOrigin(link.url) : false
  const ready = link !== null && primed?.url === link.url
  const source = !link || !embeddable ? null : ready && primed.nonce !== nonce ? withNonce(link.page_url, nonce, anchor) : link.url

  useEffect(() => {
    setLiveReady(false)
  }, [source, device])

  return (
    <section className="editorial-preview" aria-label={t('preview.title')}>
      <div className="editorial-preview-bar">
        <SegmentedControl<Device>
          aria-label={t('preview.title')}
          value={device}
          onValueChange={setDevice}
          options={[
            { value: 'desktop', label: t('preview.desktop'), icon: <Monitor aria-hidden size={14} /> },
            { value: 'mobile', label: t('preview.mobile'), icon: <Smartphone aria-hidden size={14} /> },
          ]}
        />
        {link && embeddable && loading && <Spinner aria-label={t('preview.loading')} />}
      </div>
      <div ref={stage} className="editorial-preview-stage">
        {failed && <p className="editorial-preview-message">{t('preview.failed')}</p>}
        {!failed && !link && <p className="editorial-preview-message">{t('preview.loading')}</p>}
        {link && !embeddable && (
          <div className="editorial-preview-message">
            <p>{t('preview.otherAddress')}</p>
            <Button asChild variant="outline">
              <a href={link.url} target="_blank" rel="noreferrer">
                <ExternalLink aria-hidden />
                {t('preview.open')}
              </a>
            </Button>
          </div>
        )}
        {source && (
          <div className="editorial-preview-device" style={{ width: virtualWidth * scale, height: size.height }}>
            <iframe
              key={device}
              ref={frame}
              title={t('preview.frameTitle')}
              src={source}
              onLoad={() => {
                setLoading(false)
                if (link && !ready) setPrimed({ url: link.url, nonce })
                window.setTimeout(() => {
                  if (!liveReadyRef.current) reportLive.current(false)
                }, READY_TIMEOUT_MS)
              }}
              style={{ width: virtualWidth, height: size.height / scale, transform: `scale(${scale})` }}
            />
          </div>
        )}
      </div>
    </section>
  )
}
