import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { AvailabilityPanel } from './availability/AvailabilityPanel'
import { BookingsPanel } from './bookings/BookingsPanel'
import { MeetingTypesPanel } from './meeting-types/MeetingTypesPanel'

type Section = 'availability' | 'meeting-types' | 'bookings'

export function SchedulingPage() {
  const { t } = useTranslation('scheduling')
  const [section, setSection] = useState<Section>('meeting-types')

  return (
    <div className="flex w-full min-w-0 flex-col gap-4">
      <header className="grid gap-1">
        <h1 className="text-2xl font-semibold tracking-tight">{t('page.title')}</h1>
        <p className="text-sm text-muted-foreground">{t('page.description')}</p>
      </header>
      <Tabs value={section} onValueChange={(value) => setSection(value as Section)} className="grid gap-4">
        <TabsList>
          <TabsTrigger value="meeting-types">{t('tabs.meetingTypes')}</TabsTrigger>
          <TabsTrigger value="availability">{t('tabs.availability')}</TabsTrigger>
          <TabsTrigger value="bookings">{t('tabs.bookings')}</TabsTrigger>
        </TabsList>
        <TabsContent value="meeting-types">
          <MeetingTypesPanel />
        </TabsContent>
        <TabsContent value="availability">
          <AvailabilityPanel />
        </TabsContent>
        <TabsContent value="bookings">
          <BookingsPanel />
        </TabsContent>
      </Tabs>
    </div>
  )
}
