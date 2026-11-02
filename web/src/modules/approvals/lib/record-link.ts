import {
  customObjectKey,
  customRecordPath,
  isCustomRecordType,
} from "@/features/custom-records";

const RECORD_PATHS: Record<string, string> = {
  'contacts.contact': "/contacts",
  'companies.company': "/companies",
  'deals.deal': "/deals",
  'activities.activity': "/tasks",
  'products.product': "/products",
  'quotes.quote': "/quotes",
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
