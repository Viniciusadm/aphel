const eye = '<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2.06 12.35a1 1 0 0 1 0-.7 10.75 10.75 0 0 1 19.88 0 1 1 0 0 1 0 .7 10.75 10.75 0 0 1-19.88 0"/><circle cx="12" cy="12" r="3"/></svg>'
const eyeOff = '<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M10.73 5.08A10.43 10.43 0 0 1 12 5c4.97 0 8.27 3.96 9.94 6.65a1 1 0 0 1 0 .7 13.16 13.16 0 0 1-1.67 2.68"/><path d="M6.61 6.61A13.53 13.53 0 0 0 2.06 11.65a1 1 0 0 0 0 .7C3.73 15.04 7.03 19 12 19a9.74 9.74 0 0 0 5.39-1.61"/><path d="M14.08 14.16a3 3 0 0 1-4.24-4.24"/><path d="m2 2 20 20"/></svg>'

for (const section of document.querySelectorAll('section.account')) {
  const showLabel = section.dataset.showPassword || 'Show password'
  const hideLabel = section.dataset.hidePassword || 'Hide password'
  for (const input of section.querySelectorAll('input[type=password]')) {
    const wrapper = document.createElement('span')
    wrapper.className = 'password-field'
    input.replaceWith(wrapper)
    wrapper.append(input)
    const toggle = document.createElement('button')
    toggle.type = 'button'
    toggle.className = 'password-toggle'
    const render = (visible) => {
      input.type = visible ? 'text' : 'password'
      toggle.innerHTML = visible ? eyeOff : eye
      toggle.setAttribute('aria-label', visible ? hideLabel : showLabel)
      toggle.setAttribute('aria-pressed', String(visible))
      toggle.title = visible ? hideLabel : showLabel
    }
    toggle.addEventListener('click', () => render(input.type === 'password'))
    render(false)
    wrapper.append(toggle)
  }
}
