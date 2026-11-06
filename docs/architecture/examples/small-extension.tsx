import { defineModule, useModuleTranslation } from '@aphel/web-sdk'

function GuidePage() {
  const t = useModuleTranslation('authoring_example')
  return <p>{t('guide')}</p>
}

export default defineModule({
  key: 'authoring_example',
  translations: {
    en: { guide: 'Team guidance' },
    'pt-BR': { guide: 'Orientações da equipe' },
  },
  register(app) {
    app.routes([{ path: '/authoring-example', element: <GuidePage /> }])
    app.fieldType('text', {
      Display: ({ value }) => <span>{String(value ?? '')}</span>,
    })
  },
})
