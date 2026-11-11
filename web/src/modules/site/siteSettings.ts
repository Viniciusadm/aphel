import { createContext, useContext } from 'react'
import { type OrganizationConfiguration, type SiteLink, type SiteSettings, useSaveConfiguration } from '@/sdk'

export interface SiteHeaderSettings {
  align: 'left' | 'center'
  cta: { label: string; url: string } | null
}

export interface SiteSocialLink {
  network: string
  url: string
}

export interface SiteFooterSettings {
  text: string | null
  contact: { phone: string | null; whatsapp: string | null; email: string | null; address: string | null }
  social: SiteSocialLink[]
  links: SiteLink[]
  credit: boolean
}

export interface SiteTexts {
  tagline?: string | null
  footer_text?: string | null
  cta_label?: string | null
  links?: Record<string, string>
}

export interface SiteModuleSettings extends SiteSettings {
  link_people: boolean
  people_owner: string | null
  header?: SiteHeaderSettings
  footer?: SiteFooterSettings
  translations?: Record<string, SiteTexts>
}

const DEFAULT_SETTINGS: SiteModuleSettings = {
  indexing: true,
  accounts: false,
  theme: null,
  tagline: null,
  menu: [],
  languages: [],
  link_people: false,
  people_owner: null,
  links: [],
  home: null,
  scrollbar: 'default',
}

export function currentSettings(configuration: Pick<OrganizationConfiguration, 'site'>): SiteModuleSettings {
  return { ...DEFAULT_SETTINGS, ...(configuration.site as Partial<SiteModuleSettings> | undefined) }
}

export const AppearanceFrameDraft = createContext<((values: Partial<SiteModuleSettings>) => Promise<void>) | null>(null)

export function useSaveSite<T extends Pick<OrganizationConfiguration, 'site'>>(configuration: T) {
  const save = useSaveConfiguration()
  const frameDraft = useContext(AppearanceFrameDraft)
  return (site: Partial<SiteModuleSettings>, extra: Record<string, unknown> = {}) =>
    frameDraft && (site.header || site.footer) ? frameDraft(site).then(() => ({ ...configuration, site: { ...currentSettings(configuration), ...site } })) : save({ ...extra, site: { ...currentSettings(configuration), ...site } })
}

export function languageName(code: string, locale: string): string {
  try {
    return new Intl.DisplayNames([locale], { type: 'language' }).of(code) ?? code
  } catch {
    return code
  }
}
