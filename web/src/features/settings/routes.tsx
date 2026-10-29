import type { RouteObject } from 'react-router'
import { gateModuleRoutes } from '@/kernel/modules/routes'
import type { Contribution } from '@/kernel/modules/registry'
import {
  ACCOUNT_AREA,
  DATA_AREA,
  INTEGRATIONS_AREA,
  ORGANIZATION_AREA,
  settingsEntries,
  type SettingsArea,
  type SettingsEntry,
} from '@/kernel/modules/settings'
import type { SettingsAreaHandle } from './lib/area-handle'
import { SettingsAreaLayout } from './SettingsAreaLayout'
import { SettingsAreaOverview } from './SettingsAreaOverview'

function itemRoutes({ module, value: { item } }: Contribution<SettingsEntry>): RouteObject[] {
  const page: RouteObject[] = item.lazy || item.element ? [{ path: item.path, lazy: item.lazy, element: item.element }] : []
  return gateModuleRoutes(module, [...page, ...(item.routes ?? [])])
}

function areaRoute(area: SettingsArea, extra: RouteObject[] = []): RouteObject {
  return {
    path: area.basePath.slice(1),
    element: <SettingsAreaLayout area={area} />,
    handle: { settingsArea: area } satisfies SettingsAreaHandle,
    children: [
      { index: true, element: <SettingsAreaOverview area={area} /> },
      ...settingsEntries(area.key).flatMap(itemRoutes),
      ...extra,
    ],
  }
}

export function buildSettingsRoutes(): RouteObject[] {
  return [
    areaRoute(ACCOUNT_AREA),
    areaRoute(ORGANIZATION_AREA),
    areaRoute(INTEGRATIONS_AREA),
    areaRoute(DATA_AREA),
  ]
}
