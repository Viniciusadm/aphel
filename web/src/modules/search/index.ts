import { defineModule } from '@/kernel/modules/define'
import { searchRoutes } from './ui/routes'
export default defineModule({ key: 'search', register(app) { app.routes(searchRoutes) } })
