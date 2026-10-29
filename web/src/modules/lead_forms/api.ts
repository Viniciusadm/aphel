import { keepPreviousData, useQuery, useQueryClient } from '@tanstack/react-query'
import { api, type components } from '@/api'

type Schemas = components['schemas']

export type LeadForm = Schemas['LeadFormResponse']
export type LeadFormBody = Schemas['LeadFormRequest']
export type LeadFormField = Schemas['LeadFormField']
export type LeadFormFieldKind = Schemas['LeadFormFieldKind']
export type LeadFormList = { data: LeadForm[]; meta: Schemas['PageMeta'] }
export type LeadFormOptions = Schemas['LeadFormOptions']
export type LeadFormSubmission = Schemas['LeadFormSubmission']
export type LeadFormSubmissionList = { data: LeadFormSubmission[]; meta: Schemas['PageMeta'] }
export type LeadFormSubmissionStatus = Schemas['LeadFormSubmissionStatus']

export const LEAD_FORMS_KEY = 'lead_forms'
export const MANAGE_PERMISSION = 'lead_forms.manage'
export const FIELD_KINDS: LeadFormFieldKind[] = ['text', 'email', 'phone', 'textarea', 'select', 'checkbox']
export const SUBMISSION_STATUSES: LeadFormSubmissionStatus[] = ['accepted', 'spam', 'failed']
export const SUBMISSIONS_PER_PAGE = 20
export const FULL_NAME_TARGET = 'name'

const BASE = '/v1/lead-forms'

export const leadFormsKeys = {
  all: [LEAD_FORMS_KEY] as const,
  list: () => [...leadFormsKeys.all, 'forms'] as const,
  options: () => [...leadFormsKeys.all, 'options'] as const,
  submissions: (id: string, status: LeadFormSubmissionStatus | null, page: number) =>
    [...leadFormsKeys.all, 'submissions', id, status, page] as const,
}

function formPath(id: string, action?: string) {
  const base = `${BASE}/forms/${encodeURIComponent(id)}`
  return action ? `${base}/${action}` : base
}

export const leadFormsApi = {
  list: () => api.get<LeadFormList>(`${BASE}/forms`),
  options: () => api.get<LeadFormOptions>(`${BASE}/options`),
  create: (body: LeadFormBody) => api.post<LeadForm>(`${BASE}/forms`, body),
  update: (id: string, body: LeadFormBody) => api.put<LeadForm>(formPath(id), body),
  remove: (id: string) => api.delete<void>(formPath(id)),
  rotatePublicId: (id: string) => api.post<LeadForm>(formPath(id, 'rotate-public-id')),
  submissions: (id: string, status: LeadFormSubmissionStatus | null, page: number) =>
    api.get<LeadFormSubmissionList>(formPath(id, 'submissions'), {
      query: { status, page, per_page: SUBMISSIONS_PER_PAGE },
    }),
}

export function useLeadForms() {
  return useQuery({ queryKey: leadFormsKeys.list(), queryFn: leadFormsApi.list })
}

export function useLeadFormOptions() {
  return useQuery({ queryKey: leadFormsKeys.options(), queryFn: leadFormsApi.options, staleTime: 60_000 })
}

export function useLeadFormSubmissions(id: string | null, status: LeadFormSubmissionStatus | null, page: number) {
  return useQuery({
    queryKey: leadFormsKeys.submissions(id ?? '', status, page),
    queryFn: () => leadFormsApi.submissions(id ?? '', status, page),
    enabled: id !== null,
    placeholderData: keepPreviousData,
  })
}

export function useInvalidateLeadForms() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: leadFormsKeys.all })
}

export function absoluteUrl(path: string): string {
  return `${window.location.origin}${path}`
}

export function iframeSnippet(form: LeadForm): string {
  return `<iframe src="${absoluteUrl(form.page_path)}" title="${escapeAttribute(form.name)}" style="width:100%;min-height:560px;border:0" loading="lazy"></iframe>`
}

export function scriptSnippet(form: LeadForm): string {
  return `<div data-aphel-form="${form.public_id}"></div>\n<script src="${absoluteUrl(form.embed_path)}" async></script>`
}

function escapeAttribute(value: string): string {
  return value.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
}
