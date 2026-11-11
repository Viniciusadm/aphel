import { ExternalLink, Plus, Trash2 } from 'lucide-react'
import { type ReactNode, useState } from 'react'
import type { TFunction } from 'i18next'
import { useTranslation } from 'react-i18next'
import { notifyError, notifySuccess, UnsavedChangesGuard } from '@/components/feedback'
import { PhoneFieldInput } from '@/components/fields/PhoneFieldInput'
import { formatNationalNumber, resolvePhoneCountry, splitPhoneNumber } from '@/components/fields/lib/phone'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { ToggleField } from '@/components/ui/toggle-field'
import {
  FormRow,
  type OrganizationConfiguration,
  serverFieldErrors,
  SettingsNativeSelect as NativeSelect,
  type SiteLink,
  SortableList,
} from '@/sdk'
import { useInvalidateSite, type SiteOverview } from './api'
import {
  currentSettings,
  languageName,
  type SiteFooterSettings,
  type SiteHeaderSettings,
  type SiteSocialLink,
  type SiteTexts,
  useSaveSite,
} from './siteSettings'

const TAGLINE_MAX = 160
const CTA_LABEL_MAX = 40
const FOOTER_TEXT_MAX = 500
const ADDRESS_MAX = 200
const LINK_LABEL_MAX = 60
const MAX_FOOTER_LINKS = 8
const MAX_SOCIAL_LINKS = 8
const LINK_URL = /^(\/(?!\/)\S*|https?:\/\/\S+)$/
const EMAIL = /^[^\s@<>"']+@[^\s@<>"']+\.[^\s@<>"']+$/

const SOCIAL_NETWORKS = [
  { key: 'instagram', label: 'Instagram', domains: ['instagram.com'], example: 'https://instagram.com/suaempresa' },
  { key: 'facebook', label: 'Facebook', domains: ['facebook.com', 'fb.com'], example: 'https://facebook.com/suaempresa' },
  { key: 'whatsapp', label: 'WhatsApp', domains: ['wa.me', 'whatsapp.com'], example: 'https://wa.me/5511999999999' },
  { key: 'youtube', label: 'YouTube', domains: ['youtube.com', 'youtu.be'], example: 'https://youtube.com/@suaempresa' },
  { key: 'tiktok', label: 'TikTok', domains: ['tiktok.com'], example: 'https://tiktok.com/@suaempresa' },
  { key: 'linkedin', label: 'LinkedIn', domains: ['linkedin.com'], example: 'https://linkedin.com/company/suaempresa' },
  { key: 'x', label: 'X', domains: ['x.com', 'twitter.com'], example: 'https://x.com/suaempresa' },
] as const

const DEFAULT_HEADER: SiteHeaderSettings = { align: 'left', cta: null }
const DEFAULT_FOOTER: SiteFooterSettings = {
  text: null,
  contact: { phone: null, whatsapp: null, email: null, address: null },
  social: [],
  links: [],
  credit: true,
}

interface CardProps {
  configuration: OrganizationConfiguration
  overview: SiteOverview
}

type Errors = Record<string, string>

function validLinkUrl(url: string): boolean {
  return LINK_URL.test(url.trim()) && url.trim().length <= 500
}

function socialMatches(link: SiteSocialLink): boolean {
  const network = SOCIAL_NETWORKS.find((candidate) => candidate.key === link.network)
  const match = /^https:\/\/([^/?#@:]+)/i.exec(link.url.trim())
  if (!network || !match || !validLinkUrl(link.url)) return false
  const host = match[1].toLowerCase()
  return network.domains.some((domain) => host === domain || host.endsWith(`.${domain}`))
}

function toE164(stored: string | null): string | null {
  const digits = (stored ?? '').replace(/\D/g, '')
  return digits ? `+${digits}` : null
}

function toDisplay(e164: string | null): string | null {
  if (!e164) return null
  const parts = splitPhoneNumber(e164, resolvePhoneCountry(null))
  return parts.national ? `+${parts.country.dialCode} ${formatNationalNumber(parts.country, parts.national)}` : null
}

function phoneDigitsValid(e164: string | null): boolean {
  const digits = (e164 ?? '').replace(/\D/g, '').length
  return digits === 0 || (digits >= 8 && digits <= 15)
}

function newId(): string {
  return crypto.randomUUID().replace(/-/g, '').slice(0, 12)
}

function blank(value: string): string | null {
  return value.trim() || null
}

function useLanguages(configuration: OrganizationConfiguration, overview: SiteOverview) {
  const extra = currentSettings(configuration).languages
  const [editing, setEditing] = useState<string | null>(null)
  return { extra, editing: editing && extra.includes(editing) ? editing : null, setEditing, defaultLanguage: overview.default_language }
}

function LanguagePicker({
  id,
  extra,
  editing,
  defaultLanguage,
  onChange,
}: {
  id: string
  extra: string[]
  editing: string | null
  defaultLanguage: string
  onChange: (code: string | null) => void
}) {
  const { t, i18n } = useTranslation('site')
  if (extra.length === 0) return null
  return (
    <FormRow id={id} label={t('frame.language')} hint={editing ? t('frame.translatingHint') : t('frame.languageHint')}>
      {(describedBy) => (
        <NativeSelect id={id} className="max-w-64" aria-describedby={describedBy} value={editing ?? ''} onChange={(event) => onChange(event.target.value || null)}>
          <option value="">{t('frame.defaultLanguage', { language: languageName(defaultLanguage, i18n.language) })}</option>
          {extra.map((code) => (
            <option key={code} value={code}>
              {languageName(code, i18n.language)}
            </option>
          ))}
        </NativeSelect>
      )}
    </FormRow>
  )
}

type TranslatedText = 'tagline' | 'footer_text' | 'cta_label'

function mergeTranslations(
  current: Record<string, SiteTexts>,
  languages: string[],
  edited: Record<string, SiteTexts>,
  owned: { texts: TranslatedText[]; linkIds: string[] },
): Record<string, SiteTexts> {
  const merged: Record<string, SiteTexts> = {}
  for (const code of languages) {
    const next: SiteTexts = { ...current[code] }
    const changes = edited[code] ?? {}
    for (const key of owned.texts) next[key] = changes[key]?.trim() || null
    const links = Object.fromEntries(Object.entries(next.links ?? {}).filter(([id]) => !owned.linkIds.includes(id)))
    for (const [id, label] of Object.entries(changes.links ?? {})) if (owned.linkIds.includes(id) && label.trim()) links[id] = label.trim()
    next.links = links
    merged[code] = next
  }
  return merged
}

function SaveBar({ saving, dirty, label }: { saving: boolean; dirty: boolean; label: string }) {
  return (
    <div className="flex justify-end">
      <Button type="submit" disabled={saving || !dirty}>
        {label}
      </Button>
    </div>
  )
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <fieldset className="grid gap-4 border-t pt-4">
      <legend className="float-left mb-1 w-full text-sm font-medium">{title}</legend>
      {children}
    </fieldset>
  )
}

interface HeaderDraft {
  tagline: string
  align: 'left' | 'center'
  ctaEnabled: boolean
  ctaLabel: string
  ctaUrl: string
  translations: Record<string, SiteTexts>
}

function headerDraft(configuration: OrganizationConfiguration): HeaderDraft {
  const settings = currentSettings(configuration)
  const header = { ...DEFAULT_HEADER, ...settings.header }
  return {
    tagline: settings.tagline ?? '',
    align: header.align,
    ctaEnabled: header.cta !== null,
    ctaLabel: header.cta?.label ?? '',
    ctaUrl: header.cta?.url ?? '',
    translations: settings.translations ?? {},
  }
}

function headerErrors(draft: HeaderDraft, t: TFunction<'site'>): Errors {
  const errors: Errors = {}
  if (draft.tagline.trim().length > TAGLINE_MAX) errors.tagline = t('errors.tagline')
  if (draft.ctaEnabled) {
    if (!draft.ctaLabel.trim() || draft.ctaLabel.trim().length > CTA_LABEL_MAX) errors.ctaLabel = t('errors.ctaLabel')
    if (!validLinkUrl(draft.ctaUrl)) errors.ctaUrl = t('errors.redirectTarget')
  }
  return errors
}

export function HeaderCard({ configuration, overview }: CardProps) {
  const { t } = useTranslation('site')
  const saveSite = useSaveSite(configuration)
  const invalidate = useInvalidateSite()
  const settings = currentSettings(configuration)
  const { extra, editing, setEditing, defaultLanguage } = useLanguages(configuration, overview)
  const [saved, setSaved] = useState(() => headerDraft(configuration))
  const [draft, setDraft] = useState(saved)
  const [errors, setErrors] = useState<Errors>({})
  const [saving, setSaving] = useState(false)
  const dirty = JSON.stringify(draft) !== JSON.stringify(saved)
  const menuLinks = settings.links
  const translation = editing ? (draft.translations[editing] ?? {}) : null
  const update = (changes: Partial<HeaderDraft>) => setDraft((previous) => ({ ...previous, ...changes }))
  const translate = (changes: SiteTexts) => {
    if (!editing) return
    setDraft((previous) => ({
      ...previous,
      translations: { ...previous.translations, [editing]: { ...previous.translations[editing], ...changes } },
    }))
  }

  const submit = async (event: React.FormEvent) => {
    event.preventDefault()
    const found = headerErrors(draft, t)
    setErrors(found)
    if (Object.keys(found).length > 0) return
    setSaving(true)
    try {
      const result = await saveSite({
        tagline: blank(draft.tagline),
        header: {
          align: draft.align,
          cta: draft.ctaEnabled ? { label: draft.ctaLabel.trim(), url: draft.ctaUrl.trim() } : null,
        },
        translations: mergeTranslations(settings.translations ?? {}, extra, draft.translations, { texts: ['tagline', 'cta_label'], linkIds: menuLinks.map((link) => link.id) }),
      })
      const next = headerDraft(result)
      setSaved(next)
      setDraft(next)
      void invalidate()
      notifySuccess(t('header.saved'))
    } catch (error) {
      const fields = Object.fromEntries(serverFieldErrors(error, { 'site.tagline': 'tagline', 'site.header.cta': 'ctaLabel', 'site.header.align': 'align' }))
      if (Object.keys(fields).length > 0) setErrors(Object.fromEntries(Object.keys(fields).map((key) => [key, t(key === 'tagline' ? 'errors.tagline' : 'errors.ctaLabel')])))
      notifyError(error)
    } finally {
      setSaving(false)
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('header.title')}</CardTitle>
        <CardDescription>{t('header.description')}</CardDescription>
      </CardHeader>
      <CardContent>
        <form className="grid gap-4" onSubmit={(event) => void submit(event)} noValidate>
          <UnsavedChangesGuard when={dirty} />
          <LanguagePicker id="site-header-language" extra={extra} editing={editing} defaultLanguage={defaultLanguage} onChange={setEditing} />
          <fieldset className="grid gap-4" disabled={saving}>
            <FormRow id="site-tagline" label={t('appearance.tagline')} error={errors.tagline} hint={t('appearance.taglineHint')}>
              {(describedBy) => (
                <Input
                  id="site-tagline"
                  maxLength={TAGLINE_MAX}
                  placeholder={translation ? draft.tagline || t('appearance.taglinePlaceholder') : t('appearance.taglinePlaceholder')}
                  aria-invalid={errors.tagline ? true : undefined}
                  aria-describedby={describedBy}
                  value={translation ? (translation.tagline ?? '') : draft.tagline}
                  onChange={(event) => (translation ? translate({ tagline: event.target.value }) : update({ tagline: event.target.value }))}
                />
              )}
            </FormRow>
            {!translation && (
              <FormRow id="site-header-align" label={t('header.align')} hint={t('header.alignHint')}>
                {(describedBy) => (
                  <NativeSelect
                    id="site-header-align"
                    className="max-w-64"
                    aria-describedby={describedBy}
                    value={draft.align}
                    onChange={(event) => update({ align: event.target.value === 'center' ? 'center' : 'left' })}
                  >
                    <option value="left">{t('header.alignLeft')}</option>
                    <option value="center">{t('header.alignCenter')}</option>
                  </NativeSelect>
                )}
              </FormRow>
            )}
            {!translation && (
              <ToggleField
                id="site-header-cta"
                type="switch"
                label={t('header.cta')}
                description={t('header.ctaHint')}
                checked={draft.ctaEnabled}
                onCheckedChange={(checked) => update({ ctaEnabled: checked === true })}
              />
            )}
            {draft.ctaEnabled && (
              <div className="grid gap-4 sm:grid-cols-2">
                <FormRow id="site-header-cta-label" label={t('header.ctaLabel')} error={errors.ctaLabel}>
                  {(describedBy) => (
                    <Input
                      id="site-header-cta-label"
                      maxLength={CTA_LABEL_MAX}
                      placeholder={translation ? draft.ctaLabel : t('header.ctaLabelPlaceholder')}
                      aria-invalid={errors.ctaLabel ? true : undefined}
                      aria-describedby={describedBy}
                      value={translation ? (translation.cta_label ?? '') : draft.ctaLabel}
                      onChange={(event) => (translation ? translate({ cta_label: event.target.value }) : update({ ctaLabel: event.target.value }))}
                    />
                  )}
                </FormRow>
                {!translation && (
                  <FormRow id="site-header-cta-url" label={t('menu.linkUrl')} error={errors.ctaUrl} hint={t('menu.linkUrlHint')}>
                    {(describedBy) => (
                      <Input
                        id="site-header-cta-url"
                        placeholder="https://wa.me/5511999999999"
                        autoComplete="off"
                        spellCheck={false}
                        inputMode="url"
                        aria-invalid={errors.ctaUrl ? true : undefined}
                        aria-describedby={describedBy}
                        value={draft.ctaUrl}
                        onChange={(event) => update({ ctaUrl: event.target.value })}
                      />
                    )}
                  </FormRow>
                )}
              </div>
            )}
            {translation && menuLinks.length > 0 && (
              <Section title={t('header.menuLinks')}>
                {menuLinks.map((link) => (
                  <FormRow key={link.id} id={`site-menu-link-${link.id}`} label={link.label}>
                    {(describedBy) => (
                      <Input
                        id={`site-menu-link-${link.id}`}
                        maxLength={LINK_LABEL_MAX}
                        placeholder={link.label}
                        aria-describedby={describedBy}
                        value={translation.links?.[link.id] ?? ''}
                        onChange={(event) => translate({ links: { ...translation.links, [link.id]: event.target.value } })}
                      />
                    )}
                  </FormRow>
                ))}
              </Section>
            )}
          </fieldset>
          <SaveBar saving={saving} dirty={dirty} label={t('header.save')} />
        </form>
      </CardContent>
    </Card>
  )
}

interface FooterDraft {
  text: string
  phone: string | null
  whatsapp: string | null
  email: string
  address: string
  social: SiteSocialLink[]
  links: SiteLink[]
  credit: boolean
  translations: Record<string, SiteTexts>
}

function footerDraft(configuration: OrganizationConfiguration): FooterDraft {
  const settings = currentSettings(configuration)
  const footer = { ...DEFAULT_FOOTER, ...settings.footer }
  const contact = { ...DEFAULT_FOOTER.contact, ...footer.contact }
  return {
    text: footer.text ?? '',
    phone: toE164(contact.phone),
    whatsapp: toE164(contact.whatsapp),
    email: contact.email ?? '',
    address: contact.address ?? '',
    social: footer.social,
    links: footer.links,
    credit: footer.credit,
    translations: settings.translations ?? {},
  }
}

function footerErrors(draft: FooterDraft, t: TFunction<'site'>): Errors {
  const errors: Errors = {}
  if (draft.text.trim().length > FOOTER_TEXT_MAX) errors.text = t('errors.footerText', { max: FOOTER_TEXT_MAX })
  if (!phoneDigitsValid(draft.phone)) errors.phone = t('errors.phone')
  if (!phoneDigitsValid(draft.whatsapp)) errors.whatsapp = t('errors.phone')
  if (draft.email.trim() && !EMAIL.test(draft.email.trim())) errors.email = t('errors.email')
  if (draft.address.trim().length > ADDRESS_MAX) errors.address = t('errors.address', { max: ADDRESS_MAX })
  draft.social.forEach((link, index) => {
    if (!socialMatches(link)) errors[`social.${index}`] = t('errors.social', { network: SOCIAL_NETWORKS.find((network) => network.key === link.network)?.label ?? '' })
  })
  draft.links.forEach((link, index) => {
    if (!link.label.trim() || link.label.trim().length > LINK_LABEL_MAX) errors[`links.${index}.label`] = t('errors.linkLabel')
    if (!validLinkUrl(link.url)) errors[`links.${index}.url`] = t('errors.redirectTarget')
  })
  return errors
}

const FOOTER_FIELDS: Record<string, string> = {
  'site.footer.text': 'text',
  'site.footer.contact.phone': 'phone',
  'site.footer.contact.whatsapp': 'whatsapp',
  'site.footer.contact.email': 'email',
  'site.footer.contact.address': 'address',
  'site.footer.social': 'social',
  'site.footer.links': 'links',
}

export function FooterCard({ configuration, overview }: CardProps) {
  const { t, i18n } = useTranslation('site')
  const saveSite = useSaveSite(configuration)
  const invalidate = useInvalidateSite()
  const settings = currentSettings(configuration)
  const { extra, editing, setEditing, defaultLanguage } = useLanguages(configuration, overview)
  const [saved, setSaved] = useState(() => footerDraft(configuration))
  const [draft, setDraft] = useState(saved)
  const [errors, setErrors] = useState<Errors>({})
  const [saving, setSaving] = useState(false)
  const dirty = JSON.stringify(draft) !== JSON.stringify(saved)
  const translation = editing ? (draft.translations[editing] ?? {}) : null
  const update = (changes: Partial<FooterDraft>) => setDraft((previous) => ({ ...previous, ...changes }))
  const translate = (changes: SiteTexts) => {
    if (!editing) return
    setDraft((previous) => ({
      ...previous,
      translations: { ...previous.translations, [editing]: { ...previous.translations[editing], ...changes } },
    }))
  }
  const setSocial = (index: number, changes: Partial<SiteSocialLink>) =>
    update({ social: draft.social.map((link, position) => (position === index ? { ...link, ...changes } : link)) })
  const setLink = (index: number, changes: Partial<SiteLink>) =>
    update({ links: draft.links.map((link, position) => (position === index ? { ...link, ...changes } : link)) })
  const unusedNetwork = SOCIAL_NETWORKS.find((network) => !draft.social.some((link) => link.network === network.key)) ?? SOCIAL_NETWORKS[0]

  const submit = async (event: React.FormEvent) => {
    event.preventDefault()
    const found = footerErrors(draft, t)
    setErrors(found)
    if (Object.keys(found).length > 0) return
    setSaving(true)
    try {
      const result = await saveSite({
        footer: {
          text: blank(draft.text),
          contact: {
            phone: toDisplay(draft.phone),
            whatsapp: toDisplay(draft.whatsapp),
            email: blank(draft.email),
            address: blank(draft.address),
          },
          social: draft.social.map((link) => ({ network: link.network, url: link.url.trim() })),
          links: draft.links.map((link) => ({ id: link.id, label: link.label.trim(), url: link.url.trim() })),
          credit: draft.credit,
        },
        translations: mergeTranslations(
          settings.translations ?? {},
          extra,
          draft.translations,
          { texts: ['footer_text'], linkIds: [...saved.links, ...draft.links].map((link) => link.id) },
        ),
      })
      const next = footerDraft(result)
      setSaved(next)
      setDraft(next)
      void invalidate()
      notifySuccess(t('footer.saved'))
    } catch (error) {
      const fields = serverFieldErrors(error, FOOTER_FIELDS)
      if (fields.length > 0) setErrors(Object.fromEntries(fields.map(([field]) => [field, t('errors.footerServer')])))
      notifyError(error)
    } finally {
      setSaving(false)
    }
  }

  const textCount = (translation ? (translation.footer_text ?? '') : draft.text).length

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('footer.title')}</CardTitle>
        <CardDescription>{t('footer.description')}</CardDescription>
      </CardHeader>
      <CardContent>
        <form className="grid gap-4" onSubmit={(event) => void submit(event)} noValidate>
          <UnsavedChangesGuard when={dirty} />
          <LanguagePicker id="site-footer-language" extra={extra} editing={editing} defaultLanguage={defaultLanguage} onChange={setEditing} />
          <fieldset className="grid gap-4" disabled={saving}>
            <FormRow
              id="site-footer-text"
              label={t('footer.text')}
              error={errors.text}
              hint={t('footer.textHint', { count: textCount, max: FOOTER_TEXT_MAX })}
            >
              {(describedBy) => (
                <Textarea
                  id="site-footer-text"
                  rows={3}
                  maxLength={FOOTER_TEXT_MAX}
                  placeholder={translation ? draft.text : t('footer.textPlaceholder')}
                  aria-invalid={errors.text ? true : undefined}
                  aria-describedby={describedBy}
                  value={translation ? (translation.footer_text ?? '') : draft.text}
                  onChange={(event) => (translation ? translate({ footer_text: event.target.value }) : update({ text: event.target.value }))}
                />
              )}
            </FormRow>
            {!translation && (
              <Section title={t('footer.contact')}>
                <div className="grid gap-4 sm:grid-cols-2">
                  <FormRow id="site-footer-phone" label={t('footer.phone')} error={errors.phone}>
                    {(describedBy) => (
                      <PhoneFieldInput
                        id="site-footer-phone"
                        fieldLabel={t('footer.phone')}
                        locale={i18n.language}
                        value={draft.phone}
                        invalid={Boolean(errors.phone)}
                        aria-describedby={describedBy}
                        onChange={(value) => update({ phone: value })}
                      />
                    )}
                  </FormRow>
                  <FormRow id="site-footer-whatsapp" label={t('footer.whatsapp')} error={errors.whatsapp}>
                    {(describedBy) => (
                      <PhoneFieldInput
                        id="site-footer-whatsapp"
                        fieldLabel={t('footer.whatsapp')}
                        locale={i18n.language}
                        value={draft.whatsapp}
                        invalid={Boolean(errors.whatsapp)}
                        aria-describedby={describedBy}
                        onChange={(value) => update({ whatsapp: value })}
                      />
                    )}
                  </FormRow>
                  <FormRow id="site-footer-email" label={t('footer.email')} error={errors.email}>
                    {(describedBy) => (
                      <Input
                        id="site-footer-email"
                        type="email"
                        autoComplete="off"
                        placeholder={t('footer.emailPlaceholder')}
                        aria-invalid={errors.email ? true : undefined}
                        aria-describedby={describedBy}
                        value={draft.email}
                        onChange={(event) => update({ email: event.target.value })}
                      />
                    )}
                  </FormRow>
                  <FormRow id="site-footer-address" label={t('footer.address')} error={errors.address} hint={t('footer.addressHint')}>
                    {(describedBy) => (
                      <Textarea
                        id="site-footer-address"
                        rows={2}
                        maxLength={ADDRESS_MAX}
                        placeholder={t('footer.addressPlaceholder')}
                        aria-invalid={errors.address ? true : undefined}
                        aria-describedby={describedBy}
                        value={draft.address}
                        onChange={(event) => update({ address: event.target.value })}
                      />
                    )}
                  </FormRow>
                </div>
              </Section>
            )}
            {!translation && (
              <Section title={t('footer.social')}>
                {draft.social.length === 0 && <p className="text-sm text-muted-foreground">{t('footer.socialEmpty')}</p>}
                {draft.social.map((link, index) => {
                  const network = SOCIAL_NETWORKS.find((candidate) => candidate.key === link.network) ?? SOCIAL_NETWORKS[0]
                  return (
                    <div key={index} className="grid gap-2 sm:grid-cols-[10rem_1fr_auto] sm:items-start">
                      <FormRow id={`site-social-network-${index}`} label={t('footer.network')}>
                        {(describedBy) => (
                          <NativeSelect
                            id={`site-social-network-${index}`}
                            aria-describedby={describedBy}
                            value={link.network}
                            onChange={(event) => setSocial(index, { network: event.target.value })}
                          >
                            {SOCIAL_NETWORKS.map((candidate) => (
                              <option key={candidate.key} value={candidate.key}>
                                {candidate.label}
                              </option>
                            ))}
                          </NativeSelect>
                        )}
                      </FormRow>
                      <FormRow id={`site-social-url-${index}`} label={t('footer.profileUrl')} error={errors[`social.${index}`]}>
                        {(describedBy) => (
                          <Input
                            id={`site-social-url-${index}`}
                            inputMode="url"
                            autoComplete="off"
                            spellCheck={false}
                            placeholder={network.example}
                            aria-invalid={errors[`social.${index}`] ? true : undefined}
                            aria-describedby={describedBy}
                            value={link.url}
                            onChange={(event) => setSocial(index, { url: event.target.value })}
                          />
                        )}
                      </FormRow>
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        className="sm:mt-7"
                        aria-label={t('footer.removeSocial', { network: network.label })}
                        onClick={() => update({ social: draft.social.filter((_, position) => position !== index) })}
                      >
                        <Trash2 aria-hidden="true" />
                      </Button>
                    </div>
                  )
                })}
                {draft.social.length < MAX_SOCIAL_LINKS && (
                  <div>
                    <Button type="button" variant="outline" size="sm" onClick={() => update({ social: [...draft.social, { network: unusedNetwork.key, url: '' }] })}>
                      <Plus aria-hidden="true" />
                      {t('footer.addSocial')}
                    </Button>
                  </div>
                )}
              </Section>
            )}
            <Section title={t('footer.links')}>
              {draft.links.length === 0 && <p className="text-sm text-muted-foreground">{t('footer.linksEmpty')}</p>}
              {translation
                ? draft.links.map((link) => (
                    <FormRow key={link.id} id={`site-footer-link-${link.id}`} label={link.label}>
                      {(describedBy) => (
                        <Input
                          id={`site-footer-link-${link.id}`}
                          maxLength={LINK_LABEL_MAX}
                          placeholder={link.label}
                          aria-describedby={describedBy}
                          value={translation.links?.[link.id] ?? ''}
                          onChange={(event) => translate({ links: { ...translation.links, [link.id]: event.target.value } })}
                        />
                      )}
                    </FormRow>
                  ))
                : draft.links.length > 0 && (
                    <SortableList
                      items={draft.links}
                      getId={(link) => link.id}
                      getLabel={(link) => link.label || t('footer.untitledLink')}
                      disabled={saving}
                      label={t('footer.links')}
                      onReorder={(ids) => update({ links: ids.map((id) => draft.links.find((link) => link.id === id)).filter((link): link is SiteLink => link !== undefined) })}
                      renderItem={(link, handle) => {
                        const index = draft.links.indexOf(link)
                        return (
                          <div className="flex flex-wrap items-start gap-3 rounded-md border bg-card px-3 py-2">
                            <div className="pt-7">{handle}</div>
                            <div className="grid min-w-0 flex-1 gap-3 sm:grid-cols-2">
                              <FormRow id={`site-footer-link-label-${link.id}`} label={t('menu.linkLabel')} error={errors[`links.${index}.label`]}>
                                {(describedBy) => (
                                  <Input
                                    id={`site-footer-link-label-${link.id}`}
                                    maxLength={LINK_LABEL_MAX}
                                    placeholder={t('footer.linkLabelPlaceholder')}
                                    aria-invalid={errors[`links.${index}.label`] ? true : undefined}
                                    aria-describedby={describedBy}
                                    value={link.label}
                                    onChange={(event) => setLink(index, { label: event.target.value })}
                                  />
                                )}
                              </FormRow>
                              <FormRow id={`site-footer-link-url-${link.id}`} label={t('menu.linkUrl')} error={errors[`links.${index}.url`]}>
                                {(describedBy) => (
                                  <Input
                                    id={`site-footer-link-url-${link.id}`}
                                    placeholder="/privacidade"
                                    autoComplete="off"
                                    spellCheck={false}
                                    inputMode="url"
                                    aria-invalid={errors[`links.${index}.url`] ? true : undefined}
                                    aria-describedby={describedBy}
                                    value={link.url}
                                    onChange={(event) => setLink(index, { url: event.target.value })}
                                  />
                                )}
                              </FormRow>
                            </div>
                            <Button
                              type="button"
                              variant="ghost"
                              size="sm"
                              className="mt-7"
                              aria-label={t('menu.removeLink', { label: link.label || t('footer.untitledLink') })}
                              onClick={() => update({ links: draft.links.filter((candidate) => candidate.id !== link.id) })}
                            >
                              <Trash2 aria-hidden="true" />
                            </Button>
                          </div>
                        )
                      }}
                    />
                  )}
              {!translation && draft.links.length < MAX_FOOTER_LINKS && (
                <div>
                  <Button type="button" variant="outline" size="sm" onClick={() => update({ links: [...draft.links, { id: newId(), label: '', url: '' }] })}>
                    <Plus aria-hidden="true" />
                    {t('footer.addLink')}
                  </Button>
                </div>
              )}
            </Section>
            {!translation && (
              <ToggleField
                id="site-footer-credit"
                type="switch"
                label={t('footer.credit')}
                description={t('footer.creditHint')}
                checked={draft.credit}
                onCheckedChange={(checked) => update({ credit: checked === true })}
              />
            )}
          </fieldset>
          <div className="flex flex-wrap items-center justify-between gap-2">
            <Button asChild variant="ghost" size="sm">
              <a href={overview.base_url} target="_blank" rel="noreferrer">
                <ExternalLink aria-hidden="true" />
                {t('footer.viewSite')}
              </a>
            </Button>
            <SaveBar saving={saving} dirty={dirty} label={t('footer.save')} />
          </div>
        </form>
      </CardContent>
    </Card>
  )
}
