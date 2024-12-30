use std::{
    collections::{hash_map::Entry, HashMap},
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use handlebars::{no_escape, Handlebars, HelperDef, RenderError, RenderErrorReason};
use serde_json::json;

const L10N: &[u8] = include_bytes!("l10n.json");

pub(crate) struct L10nHelper {
    templates: Handlebars<'static>,
    fallback_lang: String,
}

type Raw = HashMap<String, HashMap<String, String>>;

impl L10nHelper {
    pub(crate) fn new(custom: Option<PathBuf>, fallback_lang: String) -> Result<Self> {
        let mut l10n: Raw =
            serde_json::from_slice(L10N).context("failed to parse included l10n.json")?;
        if let Some(custom) = custom {
            let custom = read_l10n(&custom)
                .with_context(|| format!("failed to read {}", custom.to_string_lossy()))?;
            merge(&mut l10n, custom);
        }
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

fn merge(base: &mut Raw, custom: Raw) {
    for (key, custom_templates) in custom {
        match base.entry(key) {
            Entry::Occupied(mut entry) => merge_template(entry.get_mut(), custom_templates),
            Entry::Vacant(entry) => {
                entry.insert(custom_templates);
            }
        }
    }
}

fn merge_template(base: &mut HashMap<String, String>, custom_template: HashMap<String, String>) {
    for (lang, custom_template) in custom_template {
        base.insert(lang, custom_template);
    }
}

fn read_l10n(path: &Path) -> Result<Raw> {
    let reader = File::open(path)?;
    let l10n = serde_json::from_reader(reader)?;
    Ok(l10n)
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
