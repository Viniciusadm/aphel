import type { ModuleLocale, ModuleTranslations, TranslationLoader } from '@/kernel/modules/define'

const files = import.meta.glob<Record<string, unknown>>('../../i18n/*/*.json', { import: 'default' })

export const KERNEL_TRANSLATIONS: Record<string, ModuleTranslations> = {}

for (const [path, loader] of Object.entries(files)) {
  const [locale, file] = path.split('/').slice(-2)
  const namespace = file.replace(/\.json$/, '')
  KERNEL_TRANSLATIONS[namespace] = { ...KERNEL_TRANSLATIONS[namespace], [locale as ModuleLocale]: loader as TranslationLoader }
}
