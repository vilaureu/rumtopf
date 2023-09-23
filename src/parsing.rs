use std::path::Path;

use handlebars::Handlebars;
use pulldown_cmark::{escape::escape_html, html::push_html, Event, HeadingLevel, Parser, Tag};
use regex::{Captures, Regex};
use serde::Serialize;
use serde_json::json;

#[derive(Serialize)]
pub(crate) struct Recipe {
    pub(crate) title: String,
    pub(crate) short: String,
    #[serde(skip)]
    pub(crate) recipe: String,
}

pub(crate) fn parse_file(source: &Path, reg: &Handlebars) -> Recipe {
    let short = source
        .file_stem()
        .expect("file without filename")
        .to_string_lossy();

    let source = std::fs::read_to_string(source).expect("failed to read MD file");

    let mut parser = ServingWrapper::new(Parser::new(&source), reg);
    let mut recipe = String::new();
    push_html(&mut recipe, &mut parser);

    Recipe {
        title: parser.title,
        short: short.to_string(),
        recipe,
    }
}

struct ServingWrapper<'l, I> {
    iter: I,
    scaling_re: Regex,
    servings_re: Regex,
    reg: &'l Handlebars<'l>,
    title: String,
    in_title: bool,
}

impl<'l, I> ServingWrapper<'l, I> {
    pub(crate) fn new(iter: I, reg: &'l Handlebars<'l>) -> Self {
        Self {
            iter,
            scaling_re: Regex::new(r"\{\{\s*(.+)\s*\}\}").expect("failed to compile scaling regex"),
            servings_re: Regex::new(r"\{\{(.+)\s+servings?\s*\}\}")
                .expect("failed to compile servings regex"),
            reg,
            title: String::new(),
            in_title: false,
        }
    }

    fn replace(&mut self, unescaped: &str) -> String {
        if self.in_title {
            self.title.push_str(unescaped);
        }

        let mut text = String::new();
        escape_html(&mut text, unescaped).expect("failed to escape HTML");

        let text = self.servings_re.replace_all(&text, |caps: &Captures| {
            let servings: f32 = caps[1].parse().expect("parsing servings value failed");
            self.reg
                .render("servings", &json!({"servings": servings}))
                .expect("failed to render template")
        });
        let text = self.scaling_re.replace_all(&text, |caps: &Captures| {
            let base: f32 = caps[1].parse().expect("parsing base value failed");
            self.reg
                .render("scaling", &json!({"base": base}))
                .expect("failed to render template")
        });
        text.to_string()
    }
}

impl<'s, I> Iterator for ServingWrapper<'s, I>
where
    I: Iterator<Item = Event<'s>>,
{
    type Item = Event<'s>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(match self.iter.next()? {
            Event::Text(text) => Event::Html(self.replace(&text).into()),
            Event::Code(code) => {
                let code = self.replace(&code);
                Event::Html(format!("<code>{}</code>", code).into())
            }
            e if matches!(e, Event::Start(Tag::Heading(HeadingLevel::H1, _, _))) => {
                self.in_title = true;
                e
            }
            e if matches!(e, Event::End(Tag::Heading(HeadingLevel::H1, _, _))) => {
                self.in_title = false;
                e
            }
            e => e,
        })
    }
}
