use std::sync::LazyLock;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::appearance::{self, Branding, Placement, Resolved};
use crate::model::{HONEYPOT_FIELD, LeadFormFieldKind};
use crate::store::{StoredForm, public_path};
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const DEFAULT_SUCCESS_MESSAGE: &str = "Obrigado! Recebemos seus dados.";
pub(crate) const DEFAULT_SUBMIT_LABEL: &str = "Enviar";

const CSS: &str = "\
.aphel-lf-page{box-sizing:border-box;margin:0;min-height:100vh;display:flex;justify-content:center;align-items:flex-start;padding:40px 16px;background:var(--lf-page,#f3f4f6);font-family:var(--lf-font,system-ui,sans-serif);color:var(--lf-fg,#111827);-webkit-font-smoothing:antialiased}\
.aphel-lf-page[data-framed]{background:transparent;padding:0}\
.aphel-lf-page[data-framed] .aphel-lf[data-surface=card]{border:0;border-radius:0;box-shadow:none}\
.aphel-lf{box-sizing:border-box;width:100%;max-width:var(--lf-width,560px);margin:0 auto;font-family:var(--lf-font,inherit);font-size:15px;line-height:1.5;color:var(--lf-fg,inherit)}\
.aphel-lf *,.aphel-lf *::before,.aphel-lf *::after{box-sizing:border-box}\
.aphel-lf[data-surface=card]{background:var(--lf-surface,transparent);border:1px solid var(--lf-line,rgba(128,128,128,.3));border-radius:calc(var(--lf-radius,10px) + 6px);padding:32px;box-shadow:0 1px 2px rgba(16,24,40,.05),0 14px 34px -14px rgba(16,24,40,.2)}\
.aphel-lf[data-theme=dark][data-surface=card]{box-shadow:0 14px 34px -14px rgba(0,0,0,.65)}\
.aphel-lf[data-theme=outline][data-surface=card]{border-width:2px;box-shadow:none}\
.aphel-lf-head{margin-bottom:24px}\
.aphel-lf-logo{display:block;max-width:180px;max-height:44px;width:auto;height:auto;margin:0 0 18px}\
.aphel-lf-title{margin:0;font-size:24px;line-height:1.25;font-weight:700;letter-spacing:-.01em}\
.aphel-lf-intro{margin:8px 0 0;font-size:15px;color:var(--lf-muted,inherit);opacity:.9}\
.aphel-lf-field{margin-bottom:18px}\
.aphel-lf label{display:block;margin:0 0 6px;font-size:14px;font-weight:600}\
.aphel-lf-req{margin-left:2px;color:var(--lf-accent,#2563eb)}\
.aphel-lf label.aphel-lf-check{display:flex;gap:10px;align-items:flex-start;margin:0;font-weight:500;cursor:pointer}\
.aphel-lf input[type=checkbox]{flex:none;width:18px;height:18px;margin:2px 0 0;accent-color:var(--lf-accent,#2563eb)}\
.aphel-lf input[type=text],.aphel-lf input[type=email],.aphel-lf input[type=tel],.aphel-lf select,.aphel-lf textarea{display:block;width:100%;min-height:46px;margin:0;padding:10px 14px;border:1px solid var(--lf-line,rgba(128,128,128,.55));border-radius:var(--lf-radius,10px);background:var(--lf-field,transparent);color:inherit;font:inherit;transition:border-color .15s,box-shadow .15s}\
.aphel-lf[data-theme=outline] input[type=text],.aphel-lf[data-theme=outline] input[type=email],.aphel-lf[data-theme=outline] input[type=tel],.aphel-lf[data-theme=outline] select,.aphel-lf[data-theme=outline] textarea{border-width:1.5px}\
.aphel-lf textarea{min-height:124px;resize:vertical}\
.aphel-lf select{appearance:none;-webkit-appearance:none;padding-right:42px;background-image:linear-gradient(45deg,transparent 50%,currentColor 50%),linear-gradient(135deg,currentColor 50%,transparent 50%);background-position:calc(100% - 21px) 50%,calc(100% - 16px) 50%;background-size:5px 5px,5px 5px;background-repeat:no-repeat}\
.aphel-lf ::placeholder{color:var(--lf-muted,#8b95a1);opacity:.75}\
.aphel-lf input:focus,.aphel-lf select:focus,.aphel-lf textarea:focus{outline:none;border-color:var(--lf-accent,#2563eb);box-shadow:0 0 0 3px color-mix(in srgb,var(--lf-accent,#2563eb) 25%,transparent)}\
.aphel-lf input[type=checkbox]:focus-visible{outline:2px solid var(--lf-accent,#2563eb);outline-offset:2px}\
.aphel-lf input[aria-invalid=true],.aphel-lf select[aria-invalid=true],.aphel-lf textarea[aria-invalid=true]{border-color:#dc2626}\
.aphel-lf input[aria-invalid=true]:focus,.aphel-lf select[aria-invalid=true]:focus,.aphel-lf textarea[aria-invalid=true]:focus{box-shadow:0 0 0 3px color-mix(in srgb,#dc2626 22%,transparent)}\
.aphel-lf-error{margin-top:6px;font-size:13px;color:#dc2626}\
.aphel-lf[data-theme=dark] .aphel-lf-error{color:#f87171}\
.aphel-lf-alert{margin:0 0 18px;padding:12px 14px;border:1px solid color-mix(in srgb,#dc2626 30%,transparent);border-radius:var(--lf-radius,10px);background:color-mix(in srgb,#dc2626 9%,transparent);color:#b91c1c;font-size:14px}\
.aphel-lf[data-theme=dark] .aphel-lf-alert{color:#fca5a5}\
.aphel-lf-alert:empty,.aphel-lf-error:empty{display:none}\
.aphel-lf-submit{display:inline-flex;align-items:center;justify-content:center;gap:10px;min-height:48px;padding:0 28px;border:2px solid var(--lf-accent,#2563eb);border-radius:var(--lf-radius-btn,10px);background:var(--lf-accent,#2563eb);color:var(--lf-on-accent,#fff);font:inherit;font-weight:600;line-height:1;cursor:pointer;transition:filter .15s,transform .05s,box-shadow .15s}\
.aphel-lf-submit:hover{filter:brightness(.93)}\
.aphel-lf-submit:active{transform:translateY(1px)}\
.aphel-lf-submit:focus-visible{outline:none;box-shadow:0 0 0 3px color-mix(in srgb,var(--lf-accent,#2563eb) 30%,transparent)}\
.aphel-lf[data-button=outline] .aphel-lf-submit{background:transparent;color:var(--lf-accent,#2563eb)}\
.aphel-lf[data-button-width=full] .aphel-lf-submit{width:100%}\
.aphel-lf-submit[aria-busy=true],.aphel-lf-submit[disabled]{opacity:.7;cursor:progress;pointer-events:none}\
.aphel-lf-submit[aria-busy=true]::before{content:'';flex:none;width:15px;height:15px;border:2px solid currentColor;border-right-color:transparent;border-radius:50%;animation:aphel-lf-spin .7s linear infinite}\
@keyframes aphel-lf-spin{to{transform:rotate(360deg)}}\
.aphel-lf-success{display:flex;gap:16px;align-items:center;padding:8px 0}\
.aphel-lf-success-icon{flex:none;display:grid;place-items:center;width:44px;height:44px;border-radius:50%;background:color-mix(in srgb,var(--lf-accent,#2563eb) 14%,transparent);color:var(--lf-accent,#2563eb)}\
.aphel-lf-success-icon::after{content:'';width:9px;height:17px;margin-top:-3px;border:solid currentColor;border-width:0 3px 3px 0;transform:rotate(45deg)}\
.aphel-lf-success-text{margin:0;font-size:17px;font-weight:600;line-height:1.4}\
.aphel-lf-hp{position:absolute;left:-10000px;top:auto;width:1px;height:1px;overflow:hidden}\
@media (max-width:480px){.aphel-lf[data-surface=card]{padding:24px 18px}.aphel-lf-page{padding:20px 12px}}\
@media (prefers-reduced-motion:reduce){.aphel-lf-submit,.aphel-lf-submit[aria-busy=true]::before{transition:none;animation-duration:2s}}";

pub(crate) fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }
    escaped
}

pub(crate) struct PageState<'a> {
    pub(crate) values: &'a Map<String, Value>,
    pub(crate) errors: &'a [FieldViolation],
    pub(crate) message: Option<&'a str>,
    pub(crate) success: bool,
}

impl PageState<'_> {
    pub(crate) fn blank() -> PageState<'static> {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        PageState {
            values: EMPTY.get_or_init(Map::new),
            errors: &[],
            message: None,
            success: false,
        }
    }
}

fn text_value(values: &Map<String, Value>, key: &str) -> String {
    match values.get(key) {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn is_checked(values: &Map<String, Value>, key: &str) -> bool {
    match values.get(key) {
        Some(Value::Bool(flag)) => *flag,
        Some(Value::String(text)) => !text.is_empty(),
        _ => false,
    }
}

pub(crate) const PAGE_SCRIPT: &str = r"if(window.self!==window.top)document.body.setAttribute('data-framed','');document.querySelectorAll('input[type=tel]').forEach(function(i){i.addEventListener('input',function(){var v=i.value;if(v.trim().charAt(0)==='+')return;var d=v.replace(/\D/g,'').slice(0,11);var f=d;if(d.length>2){f='('+d.slice(0,2)+') '+(d.length>10?d.slice(2,7)+'-'+d.slice(7):d.length>6?d.slice(2,6)+'-'+d.slice(6):d.slice(2));}else if(d.length>0){f='('+d;}i.value=f;});});document.querySelectorAll('form').forEach(function(f){f.addEventListener('submit',function(){var b=f.querySelector('.aphel-lf-submit');if(b)setTimeout(function(){b.setAttribute('aria-busy','true');},0);});});";

pub(crate) static PAGE_SCRIPT_HASH: LazyLock<String> = LazyLock::new(|| {
    format!(
        "'sha256-{}'",
        STANDARD.encode(Sha256::digest(PAGE_SCRIPT.as_bytes()))
    )
});

const PHONE_MASK_SNIPPET: &str = r"var maskPhone = function(i){i.addEventListener('input',function(){var v=i.value;if(v.trim().charAt(0)==='+')return;var d=v.replace(/\D/g,'').slice(0,11);var f=d;if(d.length>2){f='('+d.slice(0,2)+') '+(d.length>10?d.slice(2,7)+'-'+d.slice(7):d.length>6?d.slice(2,6)+'-'+d.slice(6):d.slice(2));}else if(d.length>0){f='('+d;}i.value=f;});};";

fn head_markup(resolved: &Resolved, name: &str) -> String {
    let mut head = String::new();
    if let Some(logo) = &resolved.logo_url {
        head.push_str(&format!(
            "<img class=\"aphel-lf-logo\" src=\"{}\" alt=\"\">",
            escape(logo)
        ));
    }
    if resolved.show_title {
        head.push_str(&format!(
            "<h1 class=\"aphel-lf-title\">{}</h1>",
            escape(name)
        ));
    }
    if let Some(intro) = &resolved.intro {
        head.push_str(&format!(
            "<p class=\"aphel-lf-intro\">{}</p>",
            escape(intro)
        ));
    }
    if head.is_empty() {
        head
    } else {
        format!("<div class=\"aphel-lf-head\">{head}</div>")
    }
}

fn document(title: &str, resolved: &Resolved, inner: &str) -> String {
    format!(
        "<!doctype html><html lang=\"pt-BR\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><meta name=\"robots\" content=\"noindex\"><title>{title}</title><style>{CSS}</style></head><body class=\"aphel-lf-page\" style=\"{style}\"><main class=\"aphel-lf\"{attributes}>{inner}</main><script>{PAGE_SCRIPT}</script></body></html>",
        title = escape(title),
        style = escape(&resolved.style()),
        attributes = resolved.attribute_markup(),
    )
}

pub(crate) fn unavailable_page() -> String {
    let resolved = appearance::resolve(None, &Branding::default(), "", Placement::Page);
    document(
        "Formulário indisponível",
        &resolved,
        "<p class=\"aphel-lf-alert\">Este formulário não está disponível.</p>",
    )
}

pub(crate) fn page(form: &StoredForm, state: &PageState<'_>, branding: &Branding) -> String {
    let resolved = appearance::resolve(
        form.definition.appearance.as_ref(),
        branding,
        &form.public_id,
        Placement::Page,
    );
    let inner = if state.success {
        form_markup(form, state)
    } else {
        format!(
            "{}{}",
            head_markup(&resolved, &form.name),
            form_markup(form, state)
        )
    };
    document(&form.name, &resolved, &inner)
}

pub(crate) fn site_embed(form: &StoredForm, branding: &Branding) -> String {
    let resolved = appearance::resolve(
        form.definition.appearance.as_ref(),
        branding,
        &form.public_id,
        Placement::Embed,
    );
    format!(
        "<div class=\"site-form aphel-lf\" data-aphel-form=\"{public_id}\" data-lf=\"{public_id}\"{attributes} style=\"{style}\">{head}{markup}</div><script src=\"{script}\" defer></script>",
        public_id = escape(&form.public_id),
        attributes = resolved.attribute_markup(),
        style = escape(&resolved.style()),
        head = head_markup(&resolved, &form.name),
        markup = form_markup(form, &PageState::blank()),
        script = escape(&public_path(&form.public_id, "/embed.js")),
    )
}

fn form_markup(form: &StoredForm, state: &PageState<'_>) -> String {
    let definition = &form.definition;
    let mut body = String::new();
    if state.success {
        let message = definition
            .success_message
            .as_deref()
            .unwrap_or(DEFAULT_SUCCESS_MESSAGE);
        body.push_str(&format!(
            "<div class=\"aphel-lf-success\" role=\"status\"><span class=\"aphel-lf-success-icon\" aria-hidden=\"true\"></span><p class=\"aphel-lf-success-text\">{}</p></div>",
            escape(message)
        ));
        return body;
    }
    if let Some(message) = state.message {
        body.push_str(&format!(
            "<p class=\"aphel-lf-alert\" role=\"alert\">{}</p>",
            escape(message)
        ));
    }
    let target = if definition.redirect_url.is_some() {
        " target=\"_top\""
    } else {
        ""
    };
    body.push_str(&format!(
        "<form class=\"aphel-lf-form\" method=\"post\" action=\"{}\" accept-charset=\"UTF-8\"{target}>",
        escape(&public_path(&form.public_id, "/submissions"))
    ));
    for field in &definition.fields {
        let id = format!("f-{}", field.key);
        let required = if field.required { " required" } else { "" };
        let marker = if field.required {
            " <span class=\"aphel-lf-req\" aria-hidden=\"true\">*</span>"
        } else {
            ""
        };
        let placeholder = field
            .placeholder
            .as_deref()
            .map(|placeholder| format!(" placeholder=\"{}\"", escape(placeholder)))
            .unwrap_or_default();
        let error = state
            .errors
            .iter()
            .find(|violation| violation.field == field.key);
        let invalid = if error.is_some() {
            format!(" aria-invalid=\"true\" aria-describedby=\"{id}-error\"")
        } else {
            String::new()
        };
        let name = escape(&field.key);
        let value = escape(&text_value(state.values, &field.key));
        let max = field.kind.max_length();
        body.push_str("<div class=\"aphel-lf-field\">");
        match field.kind {
            LeadFormFieldKind::Checkbox => {
                let checked = if is_checked(state.values, &field.key) {
                    " checked"
                } else {
                    ""
                };
                body.push_str(&format!(
                    "<label class=\"aphel-lf-check\" for=\"{id}\"><input type=\"checkbox\" id=\"{id}\" name=\"{name}\" value=\"on\"{checked}{required}{invalid}> <span>{}{marker}</span></label>",
                    escape(&field.label)
                ));
            }
            LeadFormFieldKind::Textarea => body.push_str(&format!(
                "<label for=\"{id}\">{}{marker}</label><textarea id=\"{id}\" name=\"{name}\" maxlength=\"{max}\"{placeholder}{required}{invalid}>{value}</textarea>",
                escape(&field.label)
            )),
            LeadFormFieldKind::Select => {
                let current = text_value(state.values, &field.key);
                let prompt = field.placeholder.as_deref().unwrap_or("Selecione");
                let mut options = format!("<option value=\"\">{}</option>", escape(prompt));
                for option in &field.options {
                    let selected = if option.value == current {
                        " selected"
                    } else {
                        ""
                    };
                    options.push_str(&format!(
                        "<option value=\"{}\"{selected}>{}</option>",
                        escape(&option.value),
                        escape(&option.label)
                    ));
                }
                body.push_str(&format!(
                    "<label for=\"{id}\">{}{marker}</label><select id=\"{id}\" name=\"{name}\"{required}{invalid}>{options}</select>",
                    escape(&field.label)
                ));
            }
            kind => {
                let (input_type, autocomplete) = match kind {
                    LeadFormFieldKind::Email => ("email", " autocomplete=\"email\""),
                    LeadFormFieldKind::Phone => ("tel", " autocomplete=\"tel\""),
                    _ => ("text", ""),
                };
                body.push_str(&format!(
                    "<label for=\"{id}\">{}{marker}</label><input type=\"{input_type}\" id=\"{id}\" name=\"{name}\" value=\"{value}\" maxlength=\"{max}\"{autocomplete}{placeholder}{required}{invalid}>",
                    escape(&field.label)
                ));
            }
        }
        if let Some(error) = error {
            body.push_str(&format!(
                "<div class=\"aphel-lf-error\" id=\"{id}-error\">{}</div>",
                escape(&error.message)
            ));
        }
        body.push_str("</div>");
    }
    body.push_str(&format!(
        "<div class=\"aphel-lf-hp\" aria-hidden=\"true\"><label for=\"{HONEYPOT_FIELD}\">Não preencha este campo</label><input type=\"text\" id=\"{HONEYPOT_FIELD}\" name=\"{HONEYPOT_FIELD}\" tabindex=\"-1\" autocomplete=\"off\"></div>"
    ));
    body.push_str(&format!(
        "<button type=\"submit\" class=\"aphel-lf-submit\">{}</button></form>",
        escape(
            definition
                .submit_label
                .as_deref()
                .unwrap_or(DEFAULT_SUBMIT_LABEL)
        )
    ));
    body
}

const EMBED_SCRIPT: &str = r#"(function () {
  __PHONE_MASK__
  var config = __APHEL_CONFIG__;
  var script = document.currentScript;
  var base = script && script.src ? new URL(script.src, window.location.href).origin : '';
  var styleId = 'aphel-lf-style';

  function element(tag, attributes, text) {
    var node = document.createElement(tag);
    Object.keys(attributes || {}).forEach(function (name) {
      node.setAttribute(name, attributes[name]);
    });
    if (text !== undefined && text !== null) node.textContent = text;
    return node;
  }

  function injectStyle() {
    if (document.getElementById(styleId)) return;
    var style = element('style', { id: styleId });
    style.textContent = config.style;
    document.head.appendChild(style);
  }

  function readJson(response) {
    return response.json().catch(function () { return {}; }).then(function (body) {
      return { ok: response.ok, body: body };
    });
  }

  function render(container, definition) {
    container.textContent = '';
    var look = definition.appearance || {};
    if ((' ' + container.className + ' ').indexOf(' aphel-lf ') < 0) container.className = (container.className + ' aphel-lf').trim();
    container.setAttribute('data-lf', definition.public_id);
    Object.keys(look.attributes || {}).forEach(function (name) {
      container.setAttribute('data-' + name, look.attributes[name]);
    });
    Object.keys(look.vars || {}).forEach(function (name) {
      container.style.setProperty(name, look.vars[name]);
    });
    if (look.logo_url || look.title || look.intro) {
      var head = element('div', { 'class': 'aphel-lf-head' });
      if (look.logo_url) head.appendChild(element('img', { 'class': 'aphel-lf-logo', src: base + look.logo_url, alt: '' }));
      if (look.title) head.appendChild(element('h2', { 'class': 'aphel-lf-title' }, look.title));
      if (look.intro) head.appendChild(element('p', { 'class': 'aphel-lf-intro' }, look.intro));
      container.appendChild(head);
    }
    var form = element('form', { 'class': 'aphel-lf-form', novalidate: 'novalidate' });
    var errors = {};
    var inputs = {};
    definition.fields.forEach(function (field) {
      var id = 'aphel-lf-' + definition.public_id + '-' + field.key;
      var wrapper = element('div', { 'class': 'aphel-lf-field' });
      var input;
      var marker = function (label) {
        if (!field.required) return;
        label.appendChild(element('span', { 'class': 'aphel-lf-req', 'aria-hidden': 'true' }, '*'));
      };
      if (field.kind === 'checkbox') {
        input = element('input', { type: 'checkbox', id: id, name: field.key, value: 'on' });
        var check = element('label', { 'for': id, 'class': 'aphel-lf-check' });
        check.appendChild(input);
        var checkText = element('span', {}, field.label);
        marker(checkText);
        check.appendChild(checkText);
        wrapper.appendChild(check);
      } else {
        var labelNode = element('label', { 'for': id }, field.label);
        marker(labelNode);
        wrapper.appendChild(labelNode);
        if (field.kind === 'textarea') {
          input = element('textarea', { id: id, name: field.key });
        } else if (field.kind === 'select') {
          input = element('select', { id: id, name: field.key });
          input.appendChild(element('option', { value: '' }, field.placeholder || 'Selecione'));
          field.options.forEach(function (option) {
            input.appendChild(element('option', { value: option.value }, option.label));
          });
        } else {
          var type = field.kind === 'email' ? 'email' : field.kind === 'phone' ? 'tel' : 'text';
          input = element('input', { type: type, id: id, name: field.key });
          if (field.kind === 'phone') maskPhone(input);
        }
        if (field.kind !== 'select') input.setAttribute('maxlength', String(field.max_length));
        if (field.placeholder && field.kind !== 'select') input.setAttribute('placeholder', field.placeholder);
        wrapper.appendChild(input);
      }
      if (field.required) input.setAttribute('required', 'required');
      var error = element('div', { 'class': 'aphel-lf-error', id: id + '-error', 'aria-live': 'polite' });
      wrapper.appendChild(error);
      errors[field.key] = error;
      inputs[field.key] = { field: field, input: input };
      form.appendChild(wrapper);
    });
    var trap = element('div', { 'class': 'aphel-lf-hp', 'aria-hidden': 'true' });
    var trapInput = element('input', { type: 'text', name: definition.honeypot_field, tabindex: '-1', autocomplete: 'off' });
    trap.appendChild(trapInput);
    form.appendChild(trap);
    var general = element('div', { 'class': 'aphel-lf-alert', role: 'alert' });
    form.appendChild(general);
    var button = element('button', { type: 'submit', 'class': 'aphel-lf-submit' }, definition.submit_label || 'Enviar');
    form.appendChild(button);

    form.addEventListener('submit', function (event) {
      event.preventDefault();
      general.textContent = '';
      Object.keys(errors).forEach(function (key) {
        errors[key].textContent = '';
        inputs[key].input.removeAttribute('aria-invalid');
      });
      var data = {};
      Object.keys(inputs).forEach(function (key) {
        var entry = inputs[key];
        data[key] = entry.field.kind === 'checkbox' ? entry.input.checked : entry.input.value;
      });
      data[definition.honeypot_field] = trapInput.value;
      button.setAttribute('aria-busy', 'true');
      fetch(base + definition.submit_path, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
        body: JSON.stringify(data)
      }).then(readJson).then(function (result) {
        if (result.ok) {
          var accepted = (result.body && result.body.data) || {};
          if (accepted.redirect_url) {
            window.location.href = accepted.redirect_url;
            return;
          }
          container.textContent = '';
          var done = element('div', { 'class': 'aphel-lf-success', role: 'status' });
          done.appendChild(element('span', { 'class': 'aphel-lf-success-icon', 'aria-hidden': 'true' }));
          done.appendChild(element('p', { 'class': 'aphel-lf-success-text' }, accepted.success_message || config.successMessage));
          container.appendChild(done);
          return;
        }
        var failure = (result.body && result.body.error) || {};
        var fields = (failure.details && failure.details.fields) || [];
        fields.forEach(function (violation) {
          if (errors[violation.field]) {
            errors[violation.field].textContent = violation.message;
            inputs[violation.field].input.setAttribute('aria-invalid', 'true');
          }
        });
        general.textContent = failure.message || config.failureMessage;
      }).catch(function () {
        general.textContent = config.failureMessage;
      }).then(function () {
        button.removeAttribute('aria-busy');
      });
    });
    container.appendChild(form);
  }

  function mount(container) {
    if (container.getAttribute('data-aphel-form-mounted')) return;
    container.setAttribute('data-aphel-form-mounted', 'true');
    fetch(base + config.definitionPath, { headers: { Accept: 'application/json' } })
      .then(function (response) {
        if (!response.ok) throw new Error('unavailable');
        return response.json();
      })
      .then(function (body) { render(container, body.data); })
      .catch(function () { container.textContent = config.unavailableMessage; });
  }

  function start() {
    injectStyle();
    var containers = document.querySelectorAll('[data-aphel-form]');
    Array.prototype.forEach.call(containers, function (container) {
      if (container.getAttribute('data-aphel-form') === config.publicId) mount(container);
    });
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start);
  else start();
})();
"#;

pub(crate) fn embed_script(public_id: &str) -> Result<String, AppError> {
    let config = json!({
        "publicId": public_id,
        "definitionPath": public_path(public_id, ""),
        "style": CSS,
        "successMessage": DEFAULT_SUCCESS_MESSAGE,
        "failureMessage": "Não foi possível enviar. Tente novamente.",
        "unavailableMessage": "Formulário indisponível.",
    });
    let config = serde_json::to_string(&config)
        .map_err(|error| AppError::Internal(error.to_string()))?
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029");
    Ok(EMBED_SCRIPT
        .replace("__PHONE_MASK__", PHONE_MASK_SNIPPET)
        .replace("__APHEL_CONFIG__", &config))
}
