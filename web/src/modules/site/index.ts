import { Gauge, Globe, Languages, LayoutTemplate, ListTree, Palette, PanelsTopLeft, Settings2, Shuffle, UserRound } from 'lucide-react'
import { SETTINGS_MANAGE } from '@/sdk'
import { defineModule } from '@/kernel/modules/define'
import { SITE_OVERVIEW_PATH, SITE_SCREENS, siteRoutes } from './routes'

const ICONS = {
  navigation: ListTree,
  frame: PanelsTopLeft,
  appearance: Palette,
  languages: Languages,
  customers: UserRound,
  redirects: Shuffle,
  general: Settings2,
} as const

const siteModule = defineModule({
  key: 'site',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  namespaces: {
    'site-blocks': { 'pt-BR': () => import('./i18n/blocks.pt-BR.json'), en: () => import('./i18n/blocks.en.json') },
    editorial: { 'pt-BR': () => import('./i18n/editorial.pt-BR.json'), en: () => import('./i18n/editorial.en.json') },
  },
  register(app) {
    app.routes(siteRoutes)
    app.routes([{ path: 'site/editor', lazy: async () => ({ Component: (await import('./SiteEditorPage')).SiteEditorPage }) }], { layout: 'bare' })
    app.nav({
      to: '/site/editor',
      label: 'nav.editor',
      icon: LayoutTemplate,
      permission: ['site.design.edit'],
      section: 'operations',
      group: { key: 'site', label: 'nav.site', icon: Globe },
      order: 39,
    })
    app.keybindingScope({ id: 'editorial.editor', label: 'editorKeys.scope', description: 'editorKeys.scopeHelp', kind: 'overlay' })
    const editor = { scope: 'editorial.editor' }
    app.keybinding({ ...editor, id: 'editorial.save', label: 'editorKeys.save', keys: ['mod+s'], allowInEditable: true })
    app.keybinding({ ...editor, id: 'editorial.undo', label: 'editorKeys.undo', keys: ['mod+z'] })
    app.keybinding({ ...editor, id: 'editorial.redo', label: 'editorKeys.redo', keys: ['mod+shift+z', 'mod+y'] })
    app.keybinding({ ...editor, id: 'editorial.move_up', label: 'editorKeys.moveUp', keys: ['alt+arrowup'], allowInEditable: true })
    app.keybinding({ ...editor, id: 'editorial.move_down', label: 'editorKeys.moveDown', keys: ['alt+arrowdown'], allowInEditable: true })
    app.nav({
      to: SITE_OVERVIEW_PATH,
      label: 'nav.overview',
      icon: Gauge,
      permission: SETTINGS_MANAGE,
      section: 'operations',
      group: { key: 'site', label: 'nav.site', icon: Globe },
      order: 38,
    })
    SITE_SCREENS.forEach((screen, index) =>
      app.nav({
        to: screen.path,
        label: `nav.${screen.key}`,
        icon: ICONS[screen.key],
        permission: screen.key === 'appearance' || screen.key === 'frame' ? ['site.appearance.edit'] : SETTINGS_MANAGE,
        section: 'operations',
        group: { key: 'site', label: 'nav.site', icon: Globe },
        order: 41 + index,
      }),
    )
  },
})

export default siteModule
