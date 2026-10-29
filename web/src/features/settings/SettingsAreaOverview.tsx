import { useTranslation } from 'react-i18next'
import { SettingsLinkCard } from './components/SettingsLinkCard'
import { SettingsSearchInput } from './components/SettingsSearchInput'
import type { SettingsArea } from '@/kernel/modules/settings'
import { useOtherAreas } from './lib/use-other-areas'
import { useSettingsSearch } from './lib/use-settings-search'

const gridClass = 'grid gap-3 sm:grid-cols-2 xl:grid-cols-3'

interface SettingsAreaOverviewProps {
  area: SettingsArea
}

export function SettingsAreaOverview({ area }: SettingsAreaOverviewProps) {
  const { t } = useTranslation('settings')
  const { query, setQuery, filteredGroups, isSearching } = useSettingsSearch(area)
  const otherAreas = useOtherAreas(area)
  const showOtherAreas = !isSearching && area.key === 'organization' && otherAreas.length > 0

  return (
    <div className="flex flex-col gap-8">
      <SettingsSearchInput value={query} onChange={setQuery} className="max-w-md" />
      {showOtherAreas && (
        <section aria-labelledby="settings-other-areas" className="grid gap-3">
          <h2 id="settings-other-areas" className="text-sm font-semibold text-muted-foreground">
            {t('otherAreas')}
          </h2>
          <div className={gridClass}>
            {otherAreas.map((other) => (
              <SettingsLinkCard
                key={other.key}
                to={other.basePath}
                icon={other.icon}
                title={t(other.titleKey)}
                description={t(other.descriptionKey)}
              />
            ))}
          </div>
        </section>
      )}
      {isSearching && filteredGroups.length === 0 && <p className="text-sm text-muted-foreground">{t('search.empty')}</p>}
      {filteredGroups.map((group) => (
        <section key={group.key} aria-labelledby={`settings-group-${group.key}`} className="grid gap-3">
          <h2 id={`settings-group-${group.key}`} className="text-sm font-semibold text-muted-foreground">
            {group.label}
          </h2>
          <div className={gridClass}>
            {group.items.map((item) => (
              <SettingsLinkCard
                key={item.to}
                to={item.to}
                icon={item.icon}
                title={item.label}
                description={item.description}
              />
            ))}
          </div>
        </section>
      ))}
    </div>
  )
}
