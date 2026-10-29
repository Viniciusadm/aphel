import { bootModules } from '@/kernel/modules/boot'
import { webPlugins } from '@/kernel/modules/plugins'
import { MODULES } from '@/modules.generated'

bootModules([...MODULES, ...webPlugins()])
