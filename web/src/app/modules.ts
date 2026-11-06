import { bootModules } from '@/kernel/modules/boot'
import { webPlugins } from '@/kernel/modules/wasm_runtime'
import { MODULES } from '@/modules.generated'

const fixtures = import.meta.env.DEV && (import.meta.env.MODE.startsWith('frame-fixture') || import.meta.env.MODE.startsWith('experience-fixture'))
  ? (await import('@/dev/work-frame')).frameFixtureModules
  : []

const detailFixtures = import.meta.env.DEV && (import.meta.env.MODE.startsWith('detail-fixture') || import.meta.env.MODE.startsWith('experience-fixture'))
  ? (await import('@/dev/record-detail')).detailFixtureModules
  : []

bootModules([...MODULES, ...webPlugins(), ...fixtures, ...detailFixtures])
