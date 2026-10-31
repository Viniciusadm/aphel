import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { AvailabilityPanel } from './availability/AvailabilityPanel'
import { BookingsPanel } from './bookings/BookingsPanel'
import { MeetingTypesPanel } from './meeting-types/MeetingTypesPanel'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

type Section = 'availability' | 'meeting-types' | 'bookings'

export function SchedulingPage() {
  const { t } = useTranslation('scheduling')
  const [section, setSection] = useState<Section>('meeting-types')

  return (
    <Page>
      <PageHeader title={t('page.title')} description={t('page.description')} />
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
    </Page>
  )
}
