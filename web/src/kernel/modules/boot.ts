import { createModuleApp } from './app'
import type { WebModule } from './define'
import { installModuleTranslations } from './i18n'
import { installNativeFieldTypes } from './native-field-types'
import { registerModules } from './registry'

export function bootModules(modules: WebModule[]) {
  installNativeFieldTypes()
  registerModules(modules, createModuleApp)
  installModuleTranslations()
}
