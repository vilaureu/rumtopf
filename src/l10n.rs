use std::collections::HashMap;

use anyhow::{Context, Result};
use handlebars::{no_escape, Handlebars, HelperDef, RenderError, RenderErrorReason};
use serde_json::json;

const L10N: &[u8] = include_bytes!("l10n.json");

pub(crate) struct L10nHelper {
    templates: Handlebars<'static>,
    fallback_lang: String,
}

impl L10nHelper {
    pub(crate) fn new(fallback_lang: String) -> Result<Self> {
        let l10n: HashMap<String, HashMap<String, String>> =
            serde_json::from_slice(L10N).context("failed to parse included l10n.json")?;
        let mut templates = Handlebars::new();
        templates.set_strict_mode(true);
        templates.register_escape_fn(no_escape);
        for (key, v) in l10n {
            for (lang, text) in v {
                templates
                    .register_template_string(&template_name(&key, &lang), &text)
                    .with_context(|| {
                        format!("failed to register l10n template {key} for language {lang}")
                    })?;
            }
        }
        Ok(Self {
            templates,
            fallback_lang,
        })
    }
}

impl HelperDef for L10nHelper {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        helper: &handlebars::Helper<'rc>,
        _: &'reg handlebars::Handlebars<'reg>,
        ctx: &'rc handlebars::Context,
        _: &mut handlebars::RenderContext<'reg, 'rc>,
    ) -> Result<handlebars::ScopedJson<'rc>, RenderError> {
        let (key, params) = helper
            .params()
            .split_first()
            .ok_or_else(|| RenderErrorReason::Other("missing key".to_string()))?;
        let key = key
            .value()
            .as_str()
            .ok_or_else(|| RenderErrorReason::Other("key is no string".to_string()))?;

        let template = match ctx
            .data()
            .as_object()
            .and_then(|c| c.get("lang"))
            .and_then(|l| l.as_str())
        {
            Some(lang) => {
                let template = template_name(key, lang);
                if self.templates.has_template(&template) {
                    Some(template)
                } else {
                    eprintln!("missing lang {lang} for template {key}, trying fallback…");
                    None
                }
            }
            None => None,
        };

        let template = match template {
            Some(template) => template,
            None => {
                let lang = &self.fallback_lang;
                let template = template_name(key, lang);
                if self.templates.has_template(&template) {
                    template
                } else {
                    return Err(RenderErrorReason::Other(format!(
                        "missing lang {lang} for template {key}"
                    ))
                    .into());
                }
            }
        };

        let params: Vec<_> = params.iter().map(|p| p.value()).collect();
        self.templates
            .render(&template, &json!(params))
            .map(|o| json!(o).into())
    }
}

fn template_name(key: &str, lang: &str) -> String {
    format!("{key}.{lang}")
}
