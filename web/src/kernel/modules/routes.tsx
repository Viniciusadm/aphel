import type { RouteObject } from 'react-router'
import type { RouteEntry } from './app'
import { ModuleGate } from './ModuleGate'
import { contributions, isModuleRequired } from './registry'

export function gateModuleRoutes(moduleKey: string, routes: RouteObject[]): RouteObject[] {
  if (isModuleRequired(moduleKey)) return routes
  return [{ element: <ModuleGate moduleKey={moduleKey} />, children: routes }]
}

export function moduleRoutes(layout: RouteEntry['layout'] = 'app'): RouteObject[] {
  return contributions<RouteEntry>('routes')
    .filter(({ value }) => value.layout === layout)
    .flatMap(({ module, value }) => gateModuleRoutes(module, value.routes))
}

export function modulePublicRoutes(): RouteObject[] {
  return contributions<RouteObject[]>('publicRoutes').flatMap(({ value }) => value)
}
