export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  if (target.isContentEditable) return true
  const tag = target.tagName
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true
  return Boolean(target.closest('[contenteditable=""], [contenteditable="true"], [role="textbox"]'))
}

export function isInsideDialog(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && Boolean(target.closest('[role="dialog"], [role="alertdialog"], [role="menu"]'))
}
