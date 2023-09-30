mod args;
mod parsing;

use std::{
    fs::{create_dir, read_dir, File},
    io::Write,
    path::Path,
    vec,
};

use args::Args;
use clap::Parser;
use handlebars::Handlebars;
use parsing::*;
use serde_json::json;

fn main() {
    // TODO: Proper error handling.
    let args = Args::parse();
    let source = args.source;
    let destination = args.destination;

    let reg = handlebars_registry();

    create_dir(&destination).expect("cannot create destination directory");
    let recipes = process_source_dir(&source, &reg, &destination);
    for recipe in &recipes {
        write_recipe(recipe, &recipes, &reg, &destination);
    }
    create_index(recipes, &destination, &reg);
}

fn process_source_dir(source: &Path, reg: &Handlebars, destination: &Path) -> Vec<Recipe> {
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

        recipes.push(parse_file(&path, reg));
    }
    recipes
}

fn handlebars_registry() -> Handlebars<'static> {
    let mut reg = Handlebars::new();
    reg.set_strict_mode(true);

    reg.register_template_string("recipe", include_str!("templates/recipe.html"))
        .expect("failed to register template");
    reg.register_template_string("index", include_str!("templates/index.html"))
        .expect("failed to register template");
    reg.register_template_string("servings", include_str!("templates/servings.html"))
        .expect("failed to register template");
    reg.register_template_string("scaling", include_str!("templates/scaling.html"))
        .expect("failed to register template");
    reg.register_template_string("footer", include_str!("templates/footer.html"))
        .expect("failed to register template");
    reg.register_template_string("header", include_str!("templates/header.html"))
        .expect("failed to register template");

    reg
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
