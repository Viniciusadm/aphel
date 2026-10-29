import {
  customObjectKey,
  customRecordPath,
  isCustomRecordType,
} from "@/features/custom-records";

const RECORD_PATHS: Record<string, string> = {
  contact: "/contacts",
  company: "/companies",
  deal: "/deals",
  activity: "/tasks",
  product: "/products",
  quote: "/quotes",
};

export function recordPath(
  recordType: string,
  recordId: string,
): string | null {
  if (isCustomRecordType(recordType))
    return customRecordPath(customObjectKey(recordType), recordId);
  const base = RECORD_PATHS[recordType];
  return base ? `${base}/${encodeURIComponent(recordId)}` : null;
}
