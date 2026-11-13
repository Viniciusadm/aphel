import { useTranslation } from 'react-i18next'
import type { StarterTexts } from './templates'

export function useStarterTexts(): StarterTexts {
  const { t } = useTranslation('site')
  return {
    aboutHero: t('pages.starter.aboutHero'),
    aboutStoryTitle: t('pages.starter.aboutStoryTitle'),
    aboutStory: t('pages.starter.aboutStory'),
    aboutValuesTitle: t('pages.starter.aboutValuesTitle'),
    values: [
      [t('pages.starter.value1Title'), t('pages.starter.value1Text')],
      [t('pages.starter.value2Title'), t('pages.starter.value2Text')],
      [t('pages.starter.value3Title'), t('pages.starter.value3Text')],
    ],
    aboutTestimonialsTitle: t('pages.starter.aboutTestimonialsTitle'),
    servicesSubtitle: t('pages.starter.servicesSubtitle'),
    servicesTitle: t('pages.starter.servicesTitle'),
    services: [
      [t('pages.starter.service1Title'), t('pages.starter.serviceText')],
      [t('pages.starter.service2Title'), t('pages.starter.serviceText')],
      [t('pages.starter.service3Title'), t('pages.starter.serviceText')],
    ],
    faqTitle: t('pages.starter.faqTitle'),
    faq: [
      [t('pages.starter.faq1Question'), t('pages.starter.faq1Answer')],
      [t('pages.starter.faq2Question'), t('pages.starter.faq2Answer')],
    ],
    contactTitle: t('pages.starter.contactTitle'),
    contactText: t('pages.starter.contactText'),
    campaignSubtitle: t('pages.starter.campaignSubtitle'),
    campaignButton: t('pages.starter.campaignButton'),
    campaignBenefitsTitle: t('pages.starter.campaignBenefitsTitle'),
    benefits: [
      [t('pages.starter.benefit1Title'), t('pages.starter.benefitText')],
      [t('pages.starter.benefit2Title'), t('pages.starter.benefitText')],
      [t('pages.starter.benefit3Title'), t('pages.starter.benefitText')],
    ],
    campaignCall: t('pages.starter.campaignCall'),
  }
}
