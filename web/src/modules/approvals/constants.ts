import type {
  ApprovalBox,
  ApprovalMoment,
  ApprovalRule,
  ApprovalStatus,
  ApproverKind,
  ConditionKind,
} from "./types";

export const APPROVALS_PATH = "/approvals";
export const APPROVALS_SETTINGS_PATH = "/settings/approvals";
export const APPROVALS_MANAGE_PERMISSION = "approvals.manage";
export const APPROVALS_PER_PAGE = 25;
export const POLICIES_PER_PAGE = 100;
export const PENDING_COUNT_REFETCH_MS = 60_000;
export const MIN_POLICY_STEPS = 1;
export const MAX_POLICY_STEPS = 10;
export const MIN_DUE_HOURS = 1;
export const MAX_DUE_HOURS = 2160;
export const APPROVALS_TAB_KEY = "approvals";
export const QUOTE_RECORD_TYPE = "quotes.quote";
export const DEAL_RECORD_TYPE = "deals.deal";
export const STAGE_RECORD_TYPES = [DEAL_RECORD_TYPE];
export const CORE_RECORD_TYPES = [
  "people.person",
  "organizations.organization",
  "deals.deal",
  "tasks.task",
  "catalog_items.item",
] as const;

export const APPROVAL_STATUSES: ApprovalStatus[] = [
  "pending",
  "approved",
  "rejected",
  "cancelled",
];
export const APPROVAL_MOMENTS: ApprovalMoment[] = [
  "stage_change",
  "quote_send",
  "on_demand",
];
export const APPROVER_KINDS: ApproverKind[] = [
  "user",
  "role",
  "owner_team_manager",
];
export const APPROVAL_RULES: ApprovalRule[] = ["any", "all"];
export const CONDITION_KINDS: ConditionKind[] = ["always", "filter", "formula"];
export const APPROVAL_BOXES: ApprovalBox[] = [
  "pending_for_me",
  "requested_by_me",
  "all",
];

export const APPROVAL_REQUIRED = "approval_required";
export const APPROVAL_PENDING = "approval_pending";

export const APPROVAL_ERROR_CODES = [
  "approval_policy_not_found",
  "approval_policy_key_taken",
  "approval_request_not_found",
  "approval_not_approver",
  "approval_not_pending",
  "approval_required",
  "approval_pending",
  "approval_no_approver",
  "version_conflict",
  "validation_error",
] as const;

export type ApprovalErrorCode = (typeof APPROVAL_ERROR_CODES)[number];

export function approvalPath(id: string): string {
  return `${APPROVALS_PATH}/${encodeURIComponent(id)}`;
}
