import { NotebookPen } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { NoteQuickForm } from './NoteQuickForm'
import { withNotesTab } from './record-tabs'

const notesModule = defineModule({
  key: 'notes',
  required: true,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.filter('records.detailTabs', withNotesTab)
    app.quickAdd({
      key: 'note',
      label: 'quickAdd.options.note',
      title: 'quickAdd.titles.note',
      icon: NotebookPen,
      permission: 'notes.create',
      order: 30,
      render: NoteQuickForm,
    })
  },
})

export default notesModule
