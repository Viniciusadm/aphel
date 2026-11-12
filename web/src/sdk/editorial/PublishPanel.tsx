import { CalendarClock, Check, CircleAlert, Copy, Link2, Send, TriangleAlert } from 'lucide-react'
import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { toast } from '@/components/ui/toast-store'
import { fileDownloadUrl } from '@/features/files'

import type { ChecklistItem, PublishSnippet } from './document'
import { useEditorialTranslation } from './translation'

const SNIPPET_TITLE_CHARS = 60
const SNIPPET_DESCRIPTION_CHARS = 160

interface PublishPanelProps {
  open: boolean
  title: string
  items: ChecklistItem[]
  snippet: PublishSnippet
  address: string
  pageUrl: string | null
  published: boolean
  changed: boolean
  busy: boolean
  canSchedule: boolean
  onOpenChange: (open: boolean) => void
  onPublish: () => Promise<boolean>
  onSchedule: () => void
  onSettings: () => void
}

function trimmed(text: string, limit: number): string {
  return text.length > limit ? `${text.slice(0, limit - 1).trimEnd()}…` : text
}

function shareLinks(title: string, url: string): { key: string; href: string }[] {
  const text = encodeURIComponent(title)
  const link = encodeURIComponent(url)
  return [
    { key: 'whatsapp', href: `https://wa.me/?text=${text}%20${link}` },
    { key: 'x', href: `https://twitter.com/intent/tweet?text=${text}&url=${link}` },
    { key: 'linkedin', href: `https://www.linkedin.com/sharing/share-offsite/?url=${link}` },
    { key: 'email', href: `mailto:?subject=${text}&body=${link}` },
  ]
}

export function PublishPanel({ open, title, items, snippet, address, pageUrl, published, changed, busy, canSchedule, onOpenChange, onPublish, onSchedule, onSettings }: PublishPanelProps) {
  const t = useEditorialTranslation()
  const [done, setDone] = useState(false)
  const missing = items.filter((item) => !item.ok)
  const blocking = missing.filter((item) => item.required)
  const advice = missing.filter((item) => !item.required)

  function close(next: boolean) {
    onOpenChange(next)
    if (!next) setDone(false)
  }

  async function confirm() {
    if (await onPublish()) setDone(true)
    else close(false)
  }

  async function copy() {
    if (!pageUrl) return
    try {
      await navigator.clipboard.writeText(pageUrl)
      toast({ title: t('publish.copied'), variant: 'success' })
    } catch {
      toast({ title: pageUrl })
    }
  }

  return (
    <Dialog open={open} onOpenChange={close}>
      <DialogContent size="lg">
        <DialogHeader>
          <DialogTitle>{done ? t('publish.doneTitle') : changed ? t('publish.titleChanges') : t('publish.title')}</DialogTitle>
          <DialogDescription>{done ? t('publish.doneDescription') : t('publish.description')}</DialogDescription>
        </DialogHeader>
        {done ? (
          <div className="publish-done">
            {pageUrl && (
              <p className="publish-link">
                <Link2 aria-hidden size={16} />
                <a href={pageUrl} target="_blank" rel="noreferrer">
                  {pageUrl.replace(/^https?:\/\//, '')}
                </a>
                <Button type="button" variant="outline" size="sm" onClick={() => void copy()}>
                  <Copy aria-hidden />
                  {t('publish.copy')}
                </Button>
              </p>
            )}
            {pageUrl && (
              <div className="publish-share" role="group" aria-label={t('publish.share')}>
                {shareLinks(title, pageUrl).map((link) => (
                  <Button key={link.key} asChild variant="outline" size="sm">
                    <a href={link.href} target="_blank" rel="noreferrer">
                      {t(`publish.shareOn.${link.key}`)}
                    </a>
                  </Button>
                ))}
              </div>
            )}
            <div className="publish-actions">
              <Button type="button" onClick={() => close(false)}>
                {t('publish.close')}
              </Button>
            </div>
          </div>
        ) : (
          <>
            <section aria-label={t('publish.checklist')} className="publish-checklist">
              <h3>{t('publish.checklist')}</h3>
              <ul>
                {items.map((item) => (
                  <li key={item.key} data-ok={item.ok || undefined}>
                    {item.ok ? <Check aria-hidden size={16} /> : item.required ? <CircleAlert aria-hidden size={16} /> : <TriangleAlert aria-hidden size={16} />}
                    <span>{t(`checklist.${item.key}`, item.values)}</span>
                  </li>
                ))}
              </ul>
              {missing.length > 0 && (
                <Button type="button" variant="link" size="sm" onClick={() => onSettings()}>
                  {t('publish.openSettings')}
                </Button>
              )}
            </section>
            <section aria-label={t('publish.snippet')} className="publish-snippet">
              <h3>{t('publish.snippet')}</h3>
              <div className="publish-google">
                <span className="publish-google-url">{address}</span>
                <span className="publish-google-title">{trimmed(title || t('publish.noTitle'), SNIPPET_TITLE_CHARS)}</span>
                <span className="publish-google-description">{trimmed(snippet.description || t('publish.noDescription'), SNIPPET_DESCRIPTION_CHARS)}</span>
              </div>
              {snippet.image && <img className="publish-social" src={fileDownloadUrl(snippet.image)} alt="" />}
            </section>
            {blocking.length > 0 && <p className="publish-blocking" role="alert">{t('publish.blocking')}</p>}
            {advice.length > 0 && blocking.length === 0 && <p className="publish-advice">{t('publish.advice', { count: advice.length })}</p>}
            <div className="publish-actions">
              {canSchedule && (
                <Button type="button" variant="outline" disabled={busy || blocking.length > 0} onClick={() => { close(false); onSchedule() }}>
                  <CalendarClock aria-hidden />
                  {t('schedule.open')}
                </Button>
              )}
              <Button type="button" disabled={busy || blocking.length > 0} onClick={() => void confirm()}>
                <Send aria-hidden />
                {published || changed ? t('publish.confirmChanges') : t('publish.confirm')}
              </Button>
            </div>
          </>
        )}
      </DialogContent>
    </Dialog>
  )
}
