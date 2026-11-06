import { DefaultRecordPresentation } from '@/features/records/detail/DefaultRecordPresentation'
import { RECORD_PRESENTATION_CONTRACT } from '@/kernel/modules/record-presentation'
import { DefaultAppFrame } from '@/app/layout/DefaultAppFrame'
import { APP_FRAME_CONTRACT } from '@/kernel/modules/app-frame'
import {
  Building2,
  Database,
  Gauge,
  History,
  KeyRound,
  Languages,
  Puzzle,
  Server,
  Send,
  ShieldAlert,
  ShieldCheck,
  SlidersHorizontal,
  TextCursorInput,
  UserCog,
  Users,
  UsersRound,
  type LucideIcon,
} from 'lucide-react'
import type { ComponentType } from 'react'
import { type RouteObject } from 'react-router'
import { authRoutes } from '@/features/auth/routes'
import { registerCustomRecordContributions } from '@/features/custom-records/register'
import { customRecordsRoutes } from '@/features/custom-records/routes'
import { InstallerGate, setupRoutes } from '@/features/setup'
import { DefaultRecordTypeRedirect } from '@/features/settings/fields/DefaultRecordTypeRedirect'
import {
  AUDIT_READ,
  SETTINGS_MANAGE,
  SYSTEM_MANAGE,
  USERS_MANAGE,
} from '@/features/settings/lib/permissions'
import { navigationSettingsRoute } from '@/features/settings/navigation/routes'
import { systemRoutes } from '@/features/system/routes'
import { PLATFORM_SETTINGS_AREAS } from '@/kernel/modules/settings'
import { defineModule, NAV_ORDER, type ModuleApp, type SettingsAreaKey } from '@/kernel/modules/define'
import { KERNEL_TRANSLATIONS } from './translations'

function page<M, K extends keyof M>(load: () => Promise<M>, name: K): RouteObject['lazy'] {
  return async () => ({ Component: (await load())[name] as ComponentType })
}

interface SettingsItem {
  group: string
  path: string
  label: string
  icon: LucideIcon
  permission?: string
  lazy?: RouteObject['lazy']
  routes?: RouteObject[]
}

function settings(app: ModuleApp, area: SettingsAreaKey, items: SettingsItem[]) {
  for (const { label, ...item } of items) {
    app.settings(area, {
      ...item,
      label: `settings:tabs.${label}`,
      description: `settings:items.${label}.description`,
    })
  }
}

function registerNavigation(app: ModuleApp) {
  app.navSection({ key: 'work', label: 'layout:nav.groups.work', order: 10 })
  app.navSection({ key: 'records', label: 'layout:nav.groups.crm', order: 20 })
  app.navSection({ key: 'operations', label: 'layout:nav.groups.operations', order: 30 })
  app.navSection({ key: 'analysis', label: 'layout:nav.groups.analysis', order: 40 })
  app.navSection({ key: 'administration', label: 'layout:nav.groups.administration', order: 50 })
  app.nav({
    section: 'administration',
    to: '/system',
    label: 'layout:nav.system',
    icon: Server,
    permission: SYSTEM_MANAGE,
    order: NAV_ORDER.administration,
  })
}

function registerSettings(app: ModuleApp) {
  settings(app, 'account', [
    { group: 'profile', path: 'experience', label: 'personalExperience', icon: SlidersHorizontal, lazy: page(() => import('@/features/settings/personal-experience/PersonalExperiencePage'), 'PersonalExperiencePage') },
    {
      group: 'profile',
      path: 'preferences',
      label: 'preferences',
      icon: SlidersHorizontal,
      lazy: page(() => import('@/features/command-palette/preferences/PreferencesPage'), 'PreferencesPage'),
    },
    {
      group: 'profile',
      path: 'navigation',
      label: 'navigation',
      icon: Languages,
      routes: [navigationSettingsRoute],
    },
    {
      group: 'accountSecurity',
      path: 'security',
      label: 'security',
      icon: ShieldCheck,
      lazy: page(() => import('@/features/settings/security/SecurityPage'), 'SecurityPage'),
    },
    {
      group: 'accountSecurity',
      path: 'security-log',
      label: 'securityLog',
      icon: ShieldAlert,
      lazy: page(() => import('@/features/settings/security-log/SecurityLogPage'), 'SecurityLogPage'),
    },
    {
      group: 'accountSecurity',
      path: 'api-tokens',
      label: 'apiTokens',
      icon: KeyRound,
      lazy: page(() => import('@/features/settings/api-tokens/ApiTokensPage'), 'ApiTokensPage'),
    },
  ])

  settings(app, 'organization', [
    { group: 'organization', path: 'experiences', label: 'experiences', icon: SlidersHorizontal, permission: SETTINGS_MANAGE, lazy: page(() => import('@/features/settings/experiences/ExperiencesSettingsPage'), 'ExperiencesSettingsPage') },
    { group: 'organization', path: 'outgoing-email', label: 'outgoingEmail', icon: Send, permission: SETTINGS_MANAGE, lazy: page(() => import('@/features/settings/email/OutgoingEmailPage'), 'OutgoingEmailPage') },
    {
      group: 'organization',
      path: 'organization',
      label: 'organization',
      icon: Building2,
      permission: SETTINGS_MANAGE,
      lazy: page(() => import('@/features/settings/organization/OrganizationSettingsPage'), 'OrganizationSettingsPage'),
    },
    {
      group: 'organization',
      path: 'general',
      label: 'general',
      icon: Gauge,
      permission: SETTINGS_MANAGE,
      lazy: page(() => import('@/features/settings/general/GeneralSettingsPage'), 'GeneralSettingsPage'),
    },
    {
      group: 'organization',
      path: 'storage',
      label: 'storage',
      icon: Database,
      permission: SETTINGS_MANAGE,
      lazy: page(() => import('@/features/settings/storage/StorageSettingsPage'), 'StorageSettingsPage'),
    },
    {
      group: 'organization',
      path: 'users',
      label: 'users',
      icon: Users,
      permission: USERS_MANAGE,
      lazy: page(() => import('@/features/settings/users/UsersPage'), 'UsersPage'),
    },
    {
      group: 'organization',
      path: 'teams',
      label: 'teams',
      icon: UsersRound,
      permission: SETTINGS_MANAGE,
      lazy: page(() => import('@/features/settings/teams/TeamsPage'), 'TeamsPage'),
    },
    {
      group: 'organization',
      path: 'roles',
      label: 'roles',
      icon: UserCog,
      permission: SETTINGS_MANAGE,
      lazy: page(() => import('@/features/settings/roles/RolesPage'), 'RolesPage'),
    },
    {
      group: 'records',
      path: 'fields',
      label: 'fields',
      icon: TextCursorInput,
      permission: SETTINGS_MANAGE,
      routes: [
        {
          path: 'fields',
          element: <DefaultRecordTypeRedirect />,
        },
        {
          path: 'fields/:recordType',
          lazy: page(() => import('@/features/settings/fields/FieldsPage'), 'FieldsPage'),
        },
        {
          path: 'fields/:recordType/permissions',
          lazy: page(
            () => import('@/features/settings/fields/permissions/FieldPermissionsPage'),
            'FieldPermissionsPage',
          ),
        },
      ],
    },
  ])

  settings(app, 'integrations', [
    {
      group: 'connections',
      path: 'modules',
      label: 'modules',
      icon: Puzzle,
      permission: SETTINGS_MANAGE,
      lazy: page(() => import('@/features/settings/modules/ModulesPage'), 'ModulesPage'),
    },
  ])

  settings(app, 'data', [

    {
      group: 'compliance',
      path: 'audit',
      label: 'audit',
      icon: History,
      permission: AUDIT_READ,
      lazy: page(() => import('@/features/audit/AuditPage'), 'AuditPage'),
    },
  ])
}

const kernelModule = defineModule({
  key: 'kernel',
  required: true,
  namespaces: KERNEL_TRANSLATIONS,
  register(app) {
    app.provideComponent(APP_FRAME_CONTRACT, DefaultAppFrame)
    app.provideComponent(RECORD_PRESENTATION_CONTRACT, DefaultRecordPresentation)
    app.publicRoutes([...setupRoutes, { element: <InstallerGate />, children: authRoutes }])
    app.routes([
      ...customRecordsRoutes,
      ...systemRoutes,
    ])
    PLATFORM_SETTINGS_AREAS.forEach((area) => app.settingsArea(area))
    registerNavigation(app)
    registerSettings(app)
    registerCustomRecordContributions(app)
  },
})

export default kernelModule
