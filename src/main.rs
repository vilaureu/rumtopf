mod args;
mod files;
mod parsing;
mod utils;

use std::{
    collections::HashMap,
    fs::{create_dir, read_dir, remove_dir_all, DirEntry, File},
    io::Write,
    path::Path,
    process::ExitCode,
    vec,
};

use anyhow::{bail, Context, Result};
use args::Args;
use clap::Parser;
use files::*;
use handlebars::Handlebars;
use parsing::*;
use serde::Serialize;
use serde_json::json;
use utils::*;

fn main() -> Result<ExitCode> {
    let args = Args::parse();
    let reg = handlebars_registry(args.templates.as_deref())?;
    let mut ctx = Ctx {
        src: args.source,
        reg,
        dest: args.destination,
        any_error: false,
        title: args.title,
        links: args.link,
        footer: args.footer.unwrap_or_default(),
    };

    if args.remove {
        remove_dir_all(&ctx.dest).with_context(|| {
            format!(
                "Failed to remove destination directory {}",
                ctx.dest.to_string_lossy()
            )
        })?;
    }
    create_dir(&ctx.dest).with_context(|| {
        format!(
            "Failed to create destination directory {}",
            ctx.dest.to_string_lossy()
        )
    })?;
    create_static(&mut ctx);
    // Copy source files after creating static files to allow overriding them.
    let recipes = process_source_dir(&mut ctx)?;
    for recipe in &recipes {
        if let Err(err) = write_recipe(&ctx, recipe)
            .with_context(|| format!("Skipping writing recipe {}", recipe.title))
        {
            ctx.print_error(err);
        }
    }
    let by_lang = order_by_lang(&recipes);
    for lang in by_lang.by_lang.keys() {
        if let Err(err) = create_index(&ctx, by_lang.clone(), Some(lang)) {
            ctx.print_error(err);
        }
    }
    if let Err(err) = create_index(&ctx, by_lang.clone(), None) {
        ctx.print_error(err);
    }

    Ok(if ctx.any_error {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    })
}

fn process_source_dir(ctx: &mut Ctx) -> Result<Vec<Recipe>> {
    let sources = read_dir(&ctx.src).with_context(|| {
        format!(
            "Failed to read source directory {}",
            ctx.src.to_string_lossy()
        )
    })?;

    let mut recipes = vec![];
    for entry in sources {
        let entry = entry.with_context(|| {
            format!(
                "Failed to list contents of source directory {}",
                ctx.src.to_string_lossy()
            )
        });
        let recipe = entry.and_then(|entry| {
            process_source_entry(ctx, &entry).with_context(|| {
                format!("Skipping failed source {}", entry.path().to_string_lossy())
            })
        });
        let recipe = match recipe {
            Ok(r) => r,
            Err(err) => {
                ctx.print_error(err);
                continue;
            }
        };

        if let Some(recipe) = recipe {
            recipes.push(recipe);
        }
    }
    Ok(recipes)
}

fn process_source_entry(ctx: &mut Ctx, entry: &DirEntry) -> Result<Option<Recipe>> {
    let typ = entry.file_type().context("Failed to query file type")?;
    if !typ.is_file() {
        bail!("Source is not a file");
    }

    let path = entry.path();
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    {
        std::fs::copy(&path, Path::new(&ctx.dest).join(path.file_name().unwrap()))
            .context("Failed to copy file")?;
        return Ok(None);
    }

    Ok(Some(parse_file(ctx, &path)?))
}

fn handlebars_registry(override_path: Option<&Path>) -> Result<Handlebars<'static>> {
    let mut reg = Handlebars::new();
    reg.set_strict_mode(true);

    for (name, content) in TEMPLATES {
        reg.register_template_string(name, content)
            .expect("failed to register template");
    }

    if let Some(path) = override_path {
        let dir = read_dir(path).with_context(|| {
            format!(
                "Failed to read template directory {}",
                path.to_string_lossy()
            )
        })?;

        for entry in dir {
            let entry = entry.with_context(|| {
                format!(
                    "Failed to list contents of template directory {}",
                    path.to_string_lossy()
                )
            })?;
            process_template(&entry, &mut reg).with_context(|| {
                format!(
                    "Failed to process template file {}",
                    entry.path().to_string_lossy()
                )
            })?;
        }
    }

    Ok(reg)
}

fn process_template(entry: &DirEntry, reg: &mut Handlebars) -> Result<()> {
    let path = entry.path();
    let name = path
        .file_stem()
        .context("File without file name")?
        .to_string_lossy();
    if name.starts_with('.') {
        return Ok(());
    }

    let typ = entry.file_type().context("Failed to query file type")?;
    if !typ.is_file() {
        return Ok(());
    }

    reg.register_template_file(&name, &path)?;
    Ok(())
}

fn write_recipe(ctx: &Ctx, recipe: &Recipe) -> Result<()> {
    let html = render(
        &ctx.reg,
        "recipe",
        &json!({"recipe": recipe.recipe, "title": recipe.title, "ctx": template_ctx(ctx)}),
    )?;

    // short was a valid file stem so it should be safe to use as a stem here
    // too.
    let path = ctx.dest.join(recipe.stem.to_string() + ".html");
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(&path)
        .with_context(|| format!("failed to create HTML file {}", path.to_string_lossy()))?;
    file.write_all(html.as_bytes())
        .with_context(|| format!("failed to write HTML file {}", path.to_string_lossy()))?;

    Ok(())
}

type ByLang<'r> = HashMap<&'r str, Vec<&'r Recipe>>;

#[derive(Clone, Serialize)]
struct LangRecipes<'r> {
    by_lang: ByLang<'r>,
    uncat: Vec<&'r Recipe>,
}

fn order_by_lang(recipes: &[Recipe]) -> LangRecipes {
    let mut by_lang = ByLang::new();
    let mut uncat = Vec::new();
    for recipe in recipes {
        match recipe.lang {
            Some(ref lang) => by_lang.entry(lang).or_default().push(recipe),
            None => uncat.push(recipe),
        }
    }
    LangRecipes { by_lang, uncat }
}

#[derive(Serialize)]
struct IndexRecipes<'r> {
    this_lang: Vec<&'r Recipe>,
    others: LangRecipes<'r>,
}

fn create_index(ctx: &Ctx, mut lang_recipes: LangRecipes, lang: Option<&str>) -> Result<()> {
    let this_lang = match lang {
        Some(lang) => lang_recipes
            .by_lang
            .remove(lang)
            .expect("no recipes for language"),
        None => vec![],
    };
    let recipes = IndexRecipes {
        this_lang,
        others: lang_recipes,
    };

    let mut name = "index".to_string();
    if let Some(lang) = lang {
        name += ".";
        name += lang;
    }
    name += ".html";
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(ctx.dest.join(name))
        .with_context(|| "failed to create {name} file")?;

    file.write_all(
        render(
            &ctx.reg,
            "index",
            &json!({"ctx": template_ctx(ctx), "recipes": &recipes}),
        )?
        .as_bytes(),
    )
    .context("failed to write index.html file")?;

    Ok(())
}

fn template_ctx(ctx: &Ctx) -> serde_json::Value {
    json!({
        "links": ctx.links,
        "footer": ctx.footer,
        "title": ctx.title.as_deref().unwrap_or("Recipes"),
        "custom_title": ctx.title.is_some()
    })
}
