use std::sync::LazyLock;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::model::{HONEYPOT_FIELD, LeadFormFieldKind};
use crate::store::{StoredForm, public_path};
use aphel_kernel::errors::{AppError, FieldViolation};

pub(crate) const DEFAULT_SUCCESS_MESSAGE: &str = "Obrigado! Recebemos seus dados.";
pub(crate) const DEFAULT_SUBMIT_LABEL: &str = "Enviar";

const STYLE: &str = "*,*::before,*::after{box-sizing:border-box}\
body{margin:0;padding:16px;font-family:system-ui,-apple-system,'Segoe UI',Roboto,sans-serif;font-size:15px;line-height:1.45;color:#1f2328;background:transparent}\
main{max-width:560px;margin:0 auto}\
h1{font-size:20px;margin:0 0 16px}\
.field{margin-bottom:14px}\
label{display:block;font-weight:600;margin-bottom:4px}\
label.check{display:flex;gap:8px;align-items:flex-start;font-weight:400}\
input[type=text],input[type=email],input[type=tel],select,textarea{width:100%;padding:9px 10px;border:1px solid #8c959f;border-radius:6px;font:inherit;background:#fff;color:inherit}\
textarea{min-height:110px;resize:vertical}\
input:focus,select:focus,textarea:focus{outline:2px solid #0969da;outline-offset:1px}\
.error{color:#cf222e;font-size:13px;margin-top:4px}\
.alert{padding:10px 12px;border-radius:6px;margin-bottom:14px}\
.alert.error{background:#ffebe9;color:#82071e;margin-top:0}\
.alert.success{background:#dafbe1;color:#116329}\
button{background:#0969da;color:#fff;border:0;border-radius:6px;padding:10px 18px;font:inherit;font-weight:600;cursor:pointer}\
button:hover{background:#0550ae}\
.hp{position:absolute;left:-10000px;top:auto;width:1px;height:1px;overflow:hidden}";

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

pub(crate) const PHONE_MASK_SCRIPT: &str = r"document.querySelectorAll('input[type=tel]').forEach(function(i){i.addEventListener('input',function(){var v=i.value;if(v.trim().charAt(0)==='+')return;var d=v.replace(/\D/g,'').slice(0,11);var f=d;if(d.length>2){f='('+d.slice(0,2)+') '+(d.length>10?d.slice(2,7)+'-'+d.slice(7):d.length>6?d.slice(2,6)+'-'+d.slice(6):d.slice(2));}else if(d.length>0){f='('+d;}i.value=f;});});";

pub(crate) static PHONE_MASK_SCRIPT_HASH: LazyLock<String> = LazyLock::new(|| {
    format!(
        "'sha256-{}'",
        STANDARD.encode(Sha256::digest(PHONE_MASK_SCRIPT.as_bytes()))
    )
});

fn document(title: &str, body: &str) -> String {
    format!(
        "<!doctype html><html lang=\"pt-BR\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><meta name=\"robots\" content=\"noindex\"><title>{}</title><style>{STYLE}</style></head><body><main>{body}</main><script>{PHONE_MASK_SCRIPT}</script></body></html>",
        escape(title)
    )
}

pub(crate) fn unavailable_page() -> String {
    document(
        "Formulário indisponível",
        "<p class=\"alert error\">Este formulário não está disponível.</p>",
    )
}

pub(crate) fn page(form: &StoredForm, state: &PageState<'_>) -> String {
    let definition = &form.definition;
    let mut body = format!("<h1>{}</h1>", escape(&form.name));
    if state.success {
        let message = definition
            .success_message
            .as_deref()
            .unwrap_or(DEFAULT_SUCCESS_MESSAGE);
        body.push_str(&format!(
            "<p class=\"alert success\" role=\"status\">{}</p>",
            escape(message)
        ));
        return document(&form.name, &body);
    }
    if let Some(message) = state.message {
        body.push_str(&format!(
            "<p class=\"alert error\" role=\"alert\">{}</p>",
            escape(message)
        ));
    }
    let target = if definition.redirect_url.is_some() {
        " target=\"_top\""
    } else {
        ""
    };
    body.push_str(&format!(
        "<form method=\"post\" action=\"{}\" accept-charset=\"UTF-8\"{target}>",
        escape(&public_path(&form.public_id, "/submissions"))
    ));
    for field in &definition.fields {
        let id = format!("f-{}", field.key);
        let required = if field.required { " required" } else { "" };
        let marker = if field.required { " *" } else { "" };
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
        body.push_str("<div class=\"field\">");
        match field.kind {
            LeadFormFieldKind::Checkbox => {
                let checked = if is_checked(state.values, &field.key) {
                    " checked"
                } else {
                    ""
                };
                body.push_str(&format!(
                    "<label class=\"check\" for=\"{id}\"><input type=\"checkbox\" id=\"{id}\" name=\"{name}\" value=\"on\"{checked}{required}{invalid}> <span>{}{marker}</span></label>",
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
                "<div class=\"error\" id=\"{id}-error\">{}</div>",
                escape(&error.message)
            ));
        }
        body.push_str("</div>");
    }
    body.push_str(&format!(
        "<div class=\"hp\" aria-hidden=\"true\"><label for=\"{HONEYPOT_FIELD}\">Não preencha este campo</label><input type=\"text\" id=\"{HONEYPOT_FIELD}\" name=\"{HONEYPOT_FIELD}\" tabindex=\"-1\" autocomplete=\"off\"></div>"
    ));
    body.push_str(&format!(
        "<button type=\"submit\">{}</button></form>",
        escape(
            definition
                .submit_label
                .as_deref()
                .unwrap_or(DEFAULT_SUBMIT_LABEL)
        )
    ));
    document(&form.name, &body)
}

const EMBED_SCRIPT: &str = r#"(function () {
  var maskPhone = function(i){i.addEventListener('input',function(){var v=i.value;if(v.trim().charAt(0)==='+')return;var d=v.replace(/\D/g,'').slice(0,11);var f=d;if(d.length>2){f='('+d.slice(0,2)+') '+(d.length>10?d.slice(2,7)+'-'+d.slice(7):d.length>6?d.slice(2,6)+'-'+d.slice(6):d.slice(2));}else if(d.length>0){f='('+d;}i.value=f;});};
  var config = __APHEL_CONFIG__;
  var script = document.currentScript;
  var base = script && script.src ? new URL(script.src, window.location.href).origin : '';
  var styleId = 'aphel-lead-form-style';

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
    var form = element('form', { 'class': 'aphel-lf', novalidate: 'novalidate' });
    var errors = {};
    var inputs = {};
    definition.fields.forEach(function (field) {
      var id = 'aphel-lf-' + definition.public_id + '-' + field.key;
      var wrapper = element('div', { 'class': 'aphel-lf-field' });
      var input;
      var marker = field.required ? ' *' : '';
      if (field.kind === 'checkbox') {
        input = element('input', { type: 'checkbox', id: id, name: field.key, value: 'on' });
        var check = element('label', { 'for': id, 'class': 'aphel-lf-check' });
        check.appendChild(input);
        check.appendChild(element('span', {}, field.label + marker));
        wrapper.appendChild(check);
      } else {
        wrapper.appendChild(element('label', { 'for': id }, field.label + marker));
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
    var button = element('button', { type: 'submit' }, definition.submit_label || 'Enviar');
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
      button.disabled = true;
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
          container.appendChild(element('div', { 'class': 'aphel-lf-success', role: 'status' }, accepted.success_message || config.successMessage));
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
        button.disabled = false;
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

const EMBED_STYLE: &str = ".aphel-lf{font:inherit;max-width:560px}\
.aphel-lf-field{margin-bottom:14px}\
.aphel-lf label{display:block;font-weight:600;margin-bottom:4px}\
.aphel-lf label.aphel-lf-check{display:flex;gap:8px;align-items:flex-start;font-weight:400}\
.aphel-lf input[type=text],.aphel-lf input[type=email],.aphel-lf input[type=tel],.aphel-lf select,.aphel-lf textarea{width:100%;box-sizing:border-box;padding:9px 10px;border:1px solid #8c959f;border-radius:6px;font:inherit}\
.aphel-lf textarea{min-height:110px}\
.aphel-lf-error,.aphel-lf-alert{color:#cf222e;font-size:13px;margin-top:4px}\
.aphel-lf-alert:empty,.aphel-lf-error:empty{display:none}\
.aphel-lf-alert{margin-bottom:10px}\
.aphel-lf button{background:#0969da;color:#fff;border:0;border-radius:6px;padding:10px 18px;font:inherit;font-weight:600;cursor:pointer}\
.aphel-lf button[disabled]{opacity:.6;cursor:default}\
.aphel-lf-hp{position:absolute;left:-10000px;top:auto;width:1px;height:1px;overflow:hidden}\
.aphel-lf-success{padding:10px 12px;border-radius:6px;background:#dafbe1;color:#116329}";

pub(crate) fn embed_script(public_id: &str) -> Result<String, AppError> {
    let config = json!({
        "publicId": public_id,
        "definitionPath": public_path(public_id, ""),
        "style": EMBED_STYLE,
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
    Ok(EMBED_SCRIPT.replace("__APHEL_CONFIG__", &config))
}
