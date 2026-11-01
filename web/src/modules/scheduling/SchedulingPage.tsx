import { useTranslation } from 'react-i18next'
import { SectionWorkspace, SectionContent, SectionIndex, SectionLink } from '@/components/ui/section-workspace'
import { AvailabilityPanel } from './availability/AvailabilityPanel'
import { BookingsPanel } from './bookings/BookingsPanel'
import { MeetingTypesPanel } from './meeting-types/MeetingTypesPanel'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'


export function SchedulingPage() {
  const { t } = useTranslation('scheduling')

  return (
    <Page>
      <PageHeader title={t('page.title')} description={t('page.description')} />
      <SectionWorkspace defaultValue="bookings">
        <SectionIndex>
          <SectionLink value="meeting-types">{t('tabs.meetingTypes')}</SectionLink>
          <SectionLink value="availability">{t('tabs.availability')}</SectionLink>
          <SectionLink value="bookings">{t('tabs.bookings')}</SectionLink>
        </SectionIndex>
        <SectionContent value="meeting-types">
          <MeetingTypesPanel />
        </SectionContent>
        <SectionContent value="availability">
          <AvailabilityPanel />
        </SectionContent>
        <SectionContent value="bookings">
          <BookingsPanel />
        </SectionContent>
      </SectionWorkspace>
    </Page>
  )
}
