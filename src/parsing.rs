use std::{fs::read_to_string, path::Path};

use anyhow::{Context, Error, Result};
use pulldown_cmark::{html::push_html, Event, HeadingLevel, Parser, Tag, TagEnd};
use pulldown_cmark_escape::escape_html;
use regex::{Captures, Regex};
use serde::Serialize;
use serde_json::json;

use crate::utils::*;

#[derive(Serialize)]
pub(crate) struct Recipe {
    pub(crate) title: String,
    pub(crate) stem: String,
    pub(crate) short: String,
    #[serde(skip)]
    pub(crate) recipe: String,
    pub(crate) lang: Option<String>,
}

pub(crate) fn parse_file(ctx: &mut Ctx, path: &Path) -> Result<Recipe> {
    let stem = path
        .file_stem()
        .context("File without file name")?
        .to_string_lossy();
    let (short, lang) = match stem.rsplit_once(".") {
        Some((short, lang)) => (short.to_string(), Some(lang.to_string())),
        None => (stem.to_string(), None),
    };

    let source = read_to_string(path).context("Failed to read file")?;

    let mut parser = ServingWrapper::new(Parser::new(&source), ctx, path);
    let mut recipe = String::new();
    push_html(&mut recipe, &mut parser);

    Ok(Recipe {
        title: parser.title,
        stem: stem.to_string(),
        short,
        recipe,
        lang,
    })
}

struct ServingWrapper<'l, 'c, I>
where
    I: 'l,
{
    iter: I,
    scaling_re: Regex,
    servings_re: Regex,
    ctx: &'l mut Ctx<'c>,
    path: &'l Path,
    title: String,
    in_title: bool,
}

impl<'l, 'c, I> ServingWrapper<'l, 'c, I> {
    pub(crate) fn new(iter: I, ctx: &'l mut Ctx<'c>, path: &'l Path) -> Self {
        Self {
            iter,
            scaling_re: Regex::new(r"\{\{\s*([^}]+)\s*\}\}")
                .expect("failed to compile scaling regex"),
            servings_re: Regex::new(r"\{\{([^}]+)\s+servings?\s*\}\}")
                .expect("failed to compile servings regex"),
            ctx,
            path,
            title: String::new(),
            in_title: false,
        }
    }

    fn replace(&mut self, unescaped: &str) -> Result<String> {
        if self.in_title {
            self.title.push_str(unescaped);
        }

        let mut text = String::new();
        escape_html(&mut text, unescaped).context("Failed to escape HTML")?;

        let text = self.servings_re.replace_all(&text, |caps: &Captures| {
            let servings = &caps[1];
            let replacement = servings
                .parse()
                .with_context(|| format!(r#"Failed to parse servings {}"#, servings))
                .and_then(|_: f32| {
                    render(&self.ctx.reg, "servings", &json!({"servings": servings}))
                });
            match replacement {
                Ok(t) => t,
                Err(err) => {
                    Self::print_error(err, self.path, self.ctx);
                    caps[0].to_owned()
                }
            }
        });
        let text = self.scaling_re.replace_all(&text, |caps: &Captures| {
            let base = &caps[1];
            let replacement = base
                .parse()
                .with_context(|| format!(r#"Failed to parse scaling base "{}""#, base))
                .and_then(|_: f32| render(&self.ctx.reg, "scaling", &json!({"base": base})));
            match replacement {
                Ok(t) => t,
                Err(err) => {
                    Self::print_error(err, self.path, self.ctx);
                    caps[0].to_owned()
                }
            }
        });
        Ok(text.to_string())
    }

    fn replace_fallback(&mut self, unescaped: &str) -> Option<String> {
        match self.replace(unescaped) {
            Ok(u) => Some(u),
            Err(err) => {
                Self::print_error(err, self.path, self.ctx);
                None
            }
        }
    }

    fn print_error(mut err: Error, path: &Path, ctx: &mut Ctx) {
        err = err.context(format!(
            "Skipping parsing error in {}",
            path.to_string_lossy()
        ));

        ctx.print_error(err);
    }
}

impl<'l, I> Iterator for ServingWrapper<'l, '_, I>
where
    I: Iterator<Item = Event<'l>> + 'l,
{
    type Item = Event<'l>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(match self.iter.next()? {
            Event::Text(text) => 'b: {
                let Some(replaced) = self.replace_fallback(&text) else {
                    break 'b Event::Text(text);
                };

                Event::Html(replaced.into())
            }
            Event::Code(code) => 'b: {
                let Some(replaced) = self.replace_fallback(&code) else {
                    break 'b Event::Code(code);
                };

                Event::Html(format!("<code>{}</code>", replaced).into())
            }
            e if matches!(
                e,
                Event::Start(Tag::Heading {
                    level: HeadingLevel::H1,
                    ..
                })
            ) =>
            {
                self.in_title = true;
                e
            }
            e if matches!(e, Event::End(TagEnd::Heading(HeadingLevel::H1))) => {
                self.in_title = false;
                e
            }
            e => e,
        })
    }
}
