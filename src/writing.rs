use std::{collections::HashSet, fs::File, io::Write};

use anyhow::{Context, Result};
use serde_json::json;

use crate::{render, Ctx, Recipe};

pub(crate) fn write_recipes(ctx: &mut Ctx, recipes: &[Recipe]) {
    for recipe in recipes {
        if let Err(err) = write_recipe(ctx, recipe)
            .with_context(|| format!("Skipping writing recipe {}", recipe.title))
        {
            ctx.print_error(err);
        }
    }
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

pub(crate) fn write_indices(ctx: &mut Ctx<'_>, recipes: &[Recipe]) {
    let langs = recipes
        .iter()
        .map(|r| r.lang.as_deref())
        .collect::<HashSet<_>>();
    if langs.len() <= 1 {
        if let Err(err) = create_index(ctx, recipes, None) {
            ctx.print_error(err);
        }
    } else {
        for lang in langs.iter().filter_map(|r| r.as_deref()) {
            if let Err(err) = create_index(ctx, recipes, Some(lang)) {
                ctx.print_error(err);
            }
        }
        let langs = langs.iter().filter_map(|&l| l).collect::<Vec<_>>();
        if let Err(err) = write_lang_select(ctx, recipes, &langs) {
            ctx.print_error(err);
        }
    }
}

fn create_index(ctx: &Ctx, recipes: &[Recipe], lang: Option<&str>) -> Result<()> {
    let mut this_lang = Vec::new();
    let mut other_lang = Vec::new();
    for recipe in recipes {
        if lang.is_none() || recipe.lang.as_deref() == lang {
            this_lang.push(recipe);
        } else {
            other_lang.push(recipe);
        }
    }

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
            &json!({"ctx": template_ctx(ctx), "this_lang": &this_lang, "other_lang": &other_lang}),
        )?
        .as_bytes(),
    )
    .context("failed to write index.html file")?;

    Ok(())
}

fn write_lang_select(ctx: &mut Ctx<'_>, recipes: &[Recipe], langs: &[&str]) -> Result<()> {
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(ctx.dest.join("index.html"))
        .context("failed to create index.html")?;

    file.write_all(
        render(
            &ctx.reg,
            "lang",
            &json!({"ctx": template_ctx(ctx), "recipes": recipes, "langs": langs}),
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
