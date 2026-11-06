import { createElement } from 'react'
import { defineModule, RECORD_PRESENTATION_CONTRACT } from '@/sdk'
import { BrokenRecordPresentation, BrokenRecordSection, WorkRecordPresentation } from './WorkRecordPresentation'

export const detailFixtureModules = import.meta.env.MODE === 'detail-fixture-missing' ? [] : [defineModule({
  key: 'detail-fixture',
  register(app) {
    app.provideComponent(RECORD_PRESENTATION_CONTRACT, import.meta.env.MODE === 'detail-fixture-presentation-failed' ? BrokenRecordPresentation : WorkRecordPresentation)
    if (import.meta.env.MODE === 'detail-fixture-omit') {
      app.filter('records.detailTabs', (tabs) => tabs.filter((tab) => tab.key !== 'notes'))
    }
    if (import.meta.env.MODE === 'detail-fixture-failed') {
      app.filter('records.detailTabs', (tabs) => tabs.map((tab) => tab.source?.module === 'workflow' && tab.source.key === 'stage-guidance'
        ? { ...tab, content: createElement(BrokenRecordSection) }
        : tab))
    }
  },
})]
