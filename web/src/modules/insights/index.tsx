import { defineModule } from '@/kernel/modules/define'
import { lazyComponent } from '@/lib/lazy-component'
import { BirthdaysWidget } from './BirthdaysWidget'
import type { BirthdaysData, DealsBySourceData, TeamActivityRankingData } from './types'

const INSIGHTS_KEY = 'insights'

const DealsBySourceWidget = lazyComponent(async () => (await import('./DealsBySourceWidget')).DealsBySourceWidget)
const TeamActivityRankingWidget = lazyComponent(
  async () => (await import('./TeamActivityRankingWidget')).TeamActivityRankingWidget,
)

const insightsModule = defineModule({
  key: INSIGHTS_KEY,
  translations: { 'pt-BR': () => import('./i18n/pt-BR.json'), en: () => import('./i18n/en.json') },
  register(app) {
    app.widget(`${INSIGHTS_KEY}.deals_by_source`, {
      render: (data) => <DealsBySourceWidget data={data as DealsBySourceData} />,
      isEmpty: (data) => {
        const value = data as DealsBySourceData
        return value.status === 'ok' && value.total.count === 0
      },
      settings: [
        { kind: 'integer', key: 'limit', label: 'insights:widgetSettings.sourceLimit', min: 3, max: 20 },
        { kind: 'boolean', key: 'only_won', label: 'insights:widgetSettings.onlyWon' },
      ],
    })
    app.widget(`${INSIGHTS_KEY}.team_activity_ranking`, {
      render: (data) => <TeamActivityRankingWidget data={data as TeamActivityRankingData} />,
      isEmpty: (data) => (data as TeamActivityRankingData).rows.length === 0,
      settings: [
        { kind: 'integer', key: 'limit', label: 'insights:widgetSettings.rankingLimit', min: 1, max: 20 },
        { kind: 'boolean', key: 'by_type', label: 'insights:widgetSettings.byType' },
      ],
    })
    app.widget(`${INSIGHTS_KEY}.birthdays`, {
      render: (data) => <BirthdaysWidget data={data as BirthdaysData} />,
      isEmpty: (data) => {
        const value = data as BirthdaysData
        return value.status === 'ok' && value.total === 0
      },
      settings: [{ kind: 'integer', key: 'limit', label: 'insights:widgetSettings.birthdaysLimit', min: 5, max: 50 }],
    })
  },
})

export default insightsModule
