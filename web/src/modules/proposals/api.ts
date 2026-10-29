import { useQuery, useQueryClient } from '@tanstack/react-query'
import { api, type components } from '@/api'

type Schemas = components['schemas']

export type ProposalTemplate = Schemas['ProposalTemplate']
export type ProposalDocument = Schemas['ProposalDocument']
export type ProposalVariable = Schemas['ProposalVariable']
export type ProposalIssue = Schemas['ProposalTemplateIssue']
export type ProposalValidation = Schemas['ProposalValidation']
export type ProposalPreview = Schemas['ProposalPreview']
export type ProposalBlock = Schemas['ProposalBlock']
export type ProposalSpan = Schemas['ProposalSpan']
export type ProposalItemsTable = Schemas['ProposalItemsTable']
export type CreateProposalTemplate = Schemas['CreateProposalTemplateRequest']
export type UpdateProposalTemplate = Schemas['UpdateProposalTemplateRequest']
export type PreviewProposalRequest = Schemas['PreviewProposalRequest']

export const PROPOSALS_KEY = 'proposals'
export const PROPOSALS_PATH = '/proposals'
export const MANAGE_PERMISSION = 'proposals.manage'
export const GENERATE_PERMISSION = 'proposals.generate'

const BASE = '/v1/proposals'

export const proposalsKeys = {
  all: [PROPOSALS_KEY] as const,
  templates: () => [...proposalsKeys.all, 'templates'] as const,
  template: (id: string) => [...proposalsKeys.all, 'templates', id] as const,
  variables: () => [...proposalsKeys.all, 'variables'] as const,
  documents: (dealId: string) => [...proposalsKeys.all, 'documents', dealId] as const,
}

export const proposalsApi = {
  variables: () => api.get<ProposalVariable[]>(`${BASE}/variables`),
  templates: () => api.get<ProposalTemplate[]>(`${BASE}/templates`),
  template: (id: string) => api.get<ProposalTemplate>(`${BASE}/templates/${encodeURIComponent(id)}`),
  createTemplate: (body: CreateProposalTemplate) => api.post<ProposalTemplate>(`${BASE}/templates`, body),
  updateTemplate: (id: string, body: UpdateProposalTemplate) =>
    api.patch<ProposalTemplate>(`${BASE}/templates/${encodeURIComponent(id)}`, body),
  deleteTemplate: (id: string) => api.delete<void>(`${BASE}/templates/${encodeURIComponent(id)}`),
  validate: (body: string, signal?: AbortSignal) =>
    api.post<ProposalValidation>(`${BASE}/templates/validate`, { body }, { signal }),
  preview: (request: PreviewProposalRequest, signal?: AbortSignal) =>
    api.post<ProposalPreview>(`${BASE}/preview`, request, { signal }),
  documents: (dealId: string) => api.get<ProposalDocument[]>(`${BASE}/deals/${encodeURIComponent(dealId)}/documents`),
  generate: (dealId: string, templateId: string | null) =>
    api.post<ProposalDocument>(`${BASE}/deals/${encodeURIComponent(dealId)}/documents`, {
      template_id: templateId ?? undefined,
    }),
}

export function useProposalTemplates(enabled = true) {
  return useQuery({ queryKey: proposalsKeys.templates(), queryFn: proposalsApi.templates, enabled })
}

export function useProposalTemplate(id: string | undefined) {
  return useQuery({
    queryKey: proposalsKeys.template(id ?? ''),
    queryFn: () => proposalsApi.template(id!),
    enabled: Boolean(id),
  })
}

export function useProposalVariables(enabled = true) {
  return useQuery({
    queryKey: proposalsKeys.variables(),
    queryFn: proposalsApi.variables,
    staleTime: 60_000,
    enabled,
  })
}

export function useProposalDocuments(dealId: string) {
  return useQuery({ queryKey: proposalsKeys.documents(dealId), queryFn: () => proposalsApi.documents(dealId) })
}

export function useInvalidateProposals() {
  const queryClient = useQueryClient()
  return () => queryClient.invalidateQueries({ queryKey: proposalsKeys.all })
}
