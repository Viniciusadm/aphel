import type { RouteObject } from 'react-router'

export const SCORING_PATH = '/settings/scoring'

export const scoringSettingsRoute: RouteObject = {
  path: 'scoring',
  lazy: async () => ({ Component: (await import('./ScoringPage')).ScoringPage }),
}
