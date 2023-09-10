use std::{
    env::args_os,
    fs::{create_dir, read_dir, File},
    io::Write,
    path::Path,
    vec,
};

use handlebars::Handlebars;
use pulldown_cmark::{escape::escape_html, html::push_html, Event, HeadingLevel, Parser, Tag};
use regex::{Captures, Regex};
use serde::Serialize;
use serde_json::json;

fn main() {
    // TODO: Proper error handling.
    // TODO: Proper argument parsing.
    let mut args = args_os().skip(1);
    let source = args.next().expect("missing source argument");
    let destination = args.next().expect("missing destination argument");

    let mut reg = Handlebars::new();
    reg.set_strict_mode(true);
    reg.register_template_string("recipe", include_str!("recipe.html"))
        .expect("failed to register template");
    reg.register_template_string("index", include_str!("index.html"))
        .expect("failed to register template");
    reg.register_template_string("servings", include_str!("servings.html"))
        .expect("failed to register template");
    reg.register_template_string("scaling", include_str!("scaling.html"))
        .expect("failed to register template");

    create_dir(&destination).expect("cannot create destination directory");

    let source = read_dir(source).expect("failed to read source directory");
    let mut recipes = vec![];
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
            std::fs::copy(
                &path,
                Path::new(&destination).join(path.file_name().unwrap()),
            )
            .expect("cannot copy file");
            continue;
        }

        recipes.push(parse_file(&path, &reg));
    }

    for recipe in &recipes {
        write_recipe(&recipe, &recipes, &reg, destination.as_ref());
    }

    create_index(recipes, destination.as_ref(), &reg);
}

#[derive(Serialize)]
struct Recipe {
    title: String,
    short: String,
    #[serde(skip)]
    recipe: String,
}

fn parse_file(source: &Path, reg: &Handlebars) -> Recipe {
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
            scaling_re: Regex::new(r"::\s*(.+)\s*::").expect("failed to compile scaling regex"),
            servings_re: Regex::new(r"::(.+)\s+servings?\s*::")
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

fn write_recipe(recipe: &Recipe, recipes: &Vec<Recipe>, reg: &Handlebars, destination: &Path) {
    let html = reg
        .render(
            "recipe",
            &json!({"recipe": recipe.recipe, "title": recipe.title, "recipes": recipes}),
        )
        .expect("failed to render template");

    // short was a valid file stem so it should be safe to use as a stem here
    // too.
    let mut destination = File::options()
        .write(true)
        .create_new(true)
        .open(destination.join(recipe.short.to_string() + ".html"))
        .expect("failed to create HTML file");

    destination
        .write_all(html.as_bytes())
        .expect("failed to write HTML file");
}

fn create_index(recipes: Vec<Recipe>, destination: &Path, reg: &Handlebars) {
    let mut destination = File::options()
        .write(true)
        .create_new(true)
        .open(destination.join("index.html"))
        .expect("failed to create HTML file");

    destination
        .write_all(
            reg.render("index", &json!({"recipes": recipes}))
                .expect("failed to render template")
                .as_bytes(),
        )
        .expect("failed to write to HTML file");
}
