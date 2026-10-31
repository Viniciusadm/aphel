import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { zodResolver } from '@hookform/resolvers/zod'
import { useMemo } from 'react'
import { useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { z } from 'zod'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import type { PublicMeetingQuestion } from '../types'
import { Field } from '@/components/ui/field'

const MAX_ANSWER_LENGTH = 2000

function buildSchema(questions: PublicMeetingQuestion[]) {
  return z.object({
    name: z.string().trim().min(1, 'form.errors.name').max(200, 'form.errors.name'),
    email: z.string().trim().pipe(z.email('form.errors.email')),
    website: z.string(),
    answers: z.array(z.string().max(MAX_ANSWER_LENGTH, 'form.errors.answerLength')).superRefine((answers, context) => {
      questions.forEach((question, index) => {
        if (question.required && !answers[index]?.trim()) {
          context.addIssue({ code: 'custom', message: 'form.errors.required', path: [index] })
        }
      })
    }),
  })
}

export type BookingFormValues = z.infer<ReturnType<typeof buildSchema>>

export interface BookingFormSubmit {
  name: string
  email: string
  website: string
  answers: Record<string, string>
}

export type BookingFormFieldError = 'name' | 'email' | { question: string }

interface BookingFormProps {
  questions: PublicMeetingQuestion[]
  submitting: boolean
  onSubmit: (values: BookingFormSubmit) => Promise<BookingFormFieldError[] | void>
}

export function BookingForm({ questions, submitting, onSubmit }: BookingFormProps) {
  const { t } = useTranslation('scheduling', { keyPrefix: 'public' })
  const schema = useMemo(() => buildSchema(questions), [questions])
  const {
    register,
    handleSubmit,
    setError,
    formState: { errors },
  } = useForm<BookingFormValues>({
    resolver: zodResolver(schema),
    defaultValues: { name: '', email: '', website: '', answers: questions.map(() => '') },
  })
  const message = (key?: string) => (key ? t(key as 'form.errors.required') : null)

  const submit = handleSubmit(async (values) => {
    const answers: Record<string, string> = {}
    questions.forEach((question, index) => {
      const answer = values.answers[index]?.trim()
      if (answer) answers[question.key] = answer
    })
    const fieldErrors = await onSubmit({
      name: values.name.trim(),
      email: values.email.trim(),
      website: values.website,
      answers,
    })
    for (const fieldError of fieldErrors ?? []) {
      if (fieldError === 'name' || fieldError === 'email') {
        setError(fieldError, { type: 'server', message: 'form.errors.invalid' })
        continue
      }
      const index = questions.findIndex((question) => question.key === fieldError.question)
      if (index >= 0) setError(`answers.${index}`, { type: 'server', message: 'form.errors.invalid' })
    }
  })

  return (
    <form className="grid gap-4" onSubmit={submit} noValidate>
      <Field id="booking-name" label={<>{t('form.name')} <span aria-hidden>*</span></>}>
        <Input placeholder={t('placeholders.bookerName')}
          id="booking-name"
          autoComplete="name"
          aria-required
          aria-invalid={errors.name ? true : undefined}
          {...register('name')}
        />
        {errors.name && <SharedFieldError>{message(errors.name.message)}</SharedFieldError>}
      </Field>
      <Field id="booking-email" label={<>{t('form.email')} <span aria-hidden>*</span></>}>
        <Input placeholder={t('placeholders.bookerEmail')}
          id="booking-email"
          type="email"
          autoComplete="email"
          aria-required
          aria-invalid={errors.email ? true : undefined}
          {...register('email')}
        />
        {errors.email && <SharedFieldError>{message(errors.email.message)}</SharedFieldError>}
      </Field>
      {questions.map((question, index) => {
        const fieldId = `booking-question-${index}`
        const error = errors.answers?.[index]
        const Control = question.kind === 'long_text' ? Textarea : Input
        return (
          <Field key={question.key} id={fieldId} label={<>{question.label}
              {question.required && <span aria-hidden>*</span>}</>}>
            <Control
              id={fieldId}
              aria-required={question.required || undefined}
              aria-invalid={error ? true : undefined}
              {...register(`answers.${index}`)}
            />
            {error && <SharedFieldError>{message(error.message)}</SharedFieldError>}
          </Field>
        )
      })}
      <div className="absolute -left-[9999px] h-px w-px overflow-hidden" aria-hidden="true">
        <label htmlFor="booking-website">{t('form.website')}</label>
        <input id="booking-website" type="text" tabIndex={-1} autoComplete="off" {...register('website')} />
      </div>
      <Button type="submit" disabled={submitting}>
        {submitting ? t('form.submitting') : t('form.submit')}
      </Button>
    </form>
  )
}
