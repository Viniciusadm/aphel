import type { components } from '@/api/schema'

type Schemas = components['schemas']

export type OnboardingStatus = Schemas['OnboardingStatus']
export type BusinessType = Schemas['OnboardingBusinessType']
export type Offering = Schemas['OnboardingOffering']
export type TeamSize = Schemas['OnboardingTeamSize']
export type OnboardingAnswers = Schemas['OnboardingAnswersResponse']
export type TemplateSuggestion = Schemas['TemplateSuggestionResponse']
export type OnboardingRecommendation = Schemas['OnboardingRecommendationResponse']
export type OnboardingState = Schemas['OnboardingResponse']
export type SaveAnswersBody = Schemas['SaveOnboardingAnswersRequest']
export type CompleteBody = Schemas['CompleteOnboardingRequest']
