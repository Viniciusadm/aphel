import type { RouteObject } from 'react-router'

export const SCORING_PATH = '/settings/scoring'

export const scoringSettingsRoute: RouteObject = {
  path: 'scoring',
  children: [
    { index: true, lazy: async () => ({ Component: (await import('./ScoringPage')).ScoringPage }) },
    { path: ':modelId', lazy: async () => ({ Component: (await import('./ScoringModelPage')).ScoringModelPage }) },
  ],
}
