import { Target } from 'lucide-react'
import { defineModule } from '@/kernel/modules/define'
import { GoalsProgressWidget, type GoalsProgressData } from './GoalsProgressWidget'
import { goalsRoutes } from './routes'

const goalsModule = defineModule({
  key: 'goals',
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.routes(goalsRoutes)
    app.nav({ section: 'analysis',
      to: '/goals',
      label: 'nav.goals',
      icon: Target,
      permission: ['deals.read', 'tasks.read', 'people.read'],
      order: 140,
    })
    app.widget('goals_progress', {
      render: (data) => <GoalsProgressWidget data={data as GoalsProgressData} />,
      settings: [
        { kind: 'integer', key: 'limit', label: 'goals:widget.limit', min: 1, max: 20 },
        { kind: 'boolean', key: 'only_mine', label: 'goals:widget.onlyMine' },
      ],
    })
  },
})

export default goalsModule
