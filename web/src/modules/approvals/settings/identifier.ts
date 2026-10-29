const KEY_MAX_LENGTH = 64;

export function identifierFrom(text: string): string {
  const slug = text
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
  const prefixed = /^[0-9]/.test(slug) ? `policy_${slug}` : slug;
  return prefixed.slice(0, KEY_MAX_LENGTH).replace(/_+$/, "");
}
