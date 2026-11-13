import { ClipboardList } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { FORMS_KEY, MANAGE_PERMISSION } from './api'

const formsModule = defineModule({
  key: FORMS_KEY,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes([
      {
        path: 'forms',
        children: [
          { index: true, lazy: async () => ({ Component: (await import('./FormsPage')).FormsPage }) },
          { path: 'new', lazy: async () => ({ Component: (await import('./FormEditorPage')).FormEditorPage }) },
          { path: ':id/edit', lazy: async () => ({ Component: (await import('./FormEditorPage')).FormEditorPage }) },
        ],
      },
    ])
    app.nav({
      to: '/forms',
      label: 'nav',
      icon: ClipboardList,
      permission: [MANAGE_PERMISSION],
    })
  },
})

export default formsModule
