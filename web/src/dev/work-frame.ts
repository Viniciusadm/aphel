import { APP_FRAME_CONTRACT } from '@/kernel/modules/app-frame'
import { defineModule } from '@/kernel/modules/define'
import { WorkFrame } from './WorkFrame'

export const frameFixtureModules = import.meta.env.MODE === 'frame-fixture-missing' ? [] : [defineModule({
  key: 'frame-fixture',
  register(app) {
    app.provideComponent(APP_FRAME_CONTRACT, WorkFrame)
  },
})]
