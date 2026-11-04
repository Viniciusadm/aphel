import { bootModules } from '@/kernel/modules/boot'
import { webPlugins } from '@/kernel/modules/wasm_runtime'
import { MODULES } from '@/modules.generated'

bootModules([...MODULES, ...webPlugins()])
