document.addEventListener('click', async (event) => {
  const button = event.target.closest('[data-copy-url]')
  if (!button) return
  const field = button.parentElement.querySelector('input')
  field.value = window.location.href
  field.select()
  try {
    await navigator.clipboard.writeText(field.value)
    button.textContent = button.dataset.copied
  } catch {
    field.focus()
  }
})
