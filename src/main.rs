use std::{
    env::args_os,
    fs::{create_dir, read_dir, File},
    io::Write,
    path::Path,
};

use pulldown_cmark::{escape::escape_html, html::push_html, Event, HeadingLevel, Parser, Tag};
use regex::{Captures, Regex};

fn main() {
    // TODO: Proper error handling.
    // TODO: Proper argument parsing.
    let mut args = args_os().skip(1);
    let source = args.next().expect("missing source argument");
    let destination = args.next().expect("missing destination argument");

    create_dir(&destination).expect("cannot create destination directory");

    let source = read_dir(source).expect("failed to read source directory");
    for source in source {
        let source = source.expect("failed to iterate through source directory");
        let typ = source.file_type().expect("failed to query file type");
        if !typ.is_file() {
            continue;
        }

        let path = source.path();
        if !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        {
            continue;
        }

        process_file(&path, destination.as_ref());
    }
}

struct Recipe {
    name: String,
    short: String,
}

fn process_file(source: &Path, destination: &Path) {
    let short = source
        .file_stem()
        .expect("file without filename")
        .to_string_lossy();

    let source = std::fs::read_to_string(source).expect("failed to read MD file");
    let mut destination = File::options()
        .write(true)
        .create_new(true)
        .open(destination.join(short.to_string() + ".html"))
        .expect("failed to create HTML file");

    let mut parser: ServingWrapper<_> = Parser::new(&source).into();
    let mut recipe = String::new();
    push_html(&mut recipe, &mut parser);
    // TODO: Replace with proper templating.
    destination
        .write_all(
            format!(
                include_str!("template.html"),
                recipe = recipe,
                title = parser.escaped_title()
            )
            .as_bytes(),
        )
        .expect("failed to write to HTML file");
}

struct ServingWrapper<I> {
    iter: I,
    scaling_re: Regex,
    servings_re: Regex,
    title: String,
    in_title: bool,
}

impl<I> ServingWrapper<I> {
    pub(crate) fn escaped_title(&self) -> String {
        let mut title = String::new();
        escape_html(&mut title, &self.title).expect("failed to escape HTML");
        title
    }

    fn replace(&mut self, unescaped: &str) -> String {
        if self.in_title {
            self.title.push_str(unescaped);
        }

        let mut text = String::new();
        escape_html(&mut text, unescaped).expect("failed to escape HTML");

        let text = self.servings_re.replace_all(&text, |caps: &Captures| {
            let servings: f32 = caps[1].parse().expect("parsing servings value failed");
            format!(include_str!("servings.html"), servings)
        });
        let text = self.scaling_re.replace_all(&text, |caps: &Captures| {
            let base: f32 = caps[1].parse().expect("parsing base value failed");
            format!(r#"<span class="scaling" data-base="{0}">{0}</span>"#, base)
        });
        text.to_string()
    }
}

impl<I> From<I> for ServingWrapper<I> {
    fn from(value: I) -> Self {
        Self {
            iter: value,
            scaling_re: Regex::new(r"::\s*(.+)\s*::").expect("failed to compile scaling regex"),
            servings_re: Regex::new(r"::(.+)\s+servings?\s*::")
                .expect("failed to compile servings regex"),
            title: String::new(),
            in_title: false,
        }
    }
}

impl<'s, I> Iterator for ServingWrapper<I>
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
