import i18n, { DEFAULT_LANGUAGE } from '@/i18n'

const NAMESPACE = 'booking'

type Resources = Record<string, unknown>

const loaders: Record<string, () => Promise<{ default: Resources }>> = {
  'pt-BR': () => import('../i18n/pt-BR.json'),
  en: () => import('../i18n/en.json'),
}

export async function ensureBookingTranslations(language: string = i18n.resolvedLanguage ?? i18n.language) {
  const locales = [...new Set([language, DEFAULT_LANGUAGE])].filter((locale) => locale in loaders)
  await Promise.all(
    locales.map(async (locale) => {
      if (i18n.hasResourceBundle(locale, NAMESPACE)) return
      const { default: resources } = await loaders[locale]()
      i18n.addResourceBundle(locale, NAMESPACE, resources, true, true)
    }),
  )
}
