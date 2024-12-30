use std::{collections::HashSet, fs::File, io::Write};

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::json;

use crate::{render, Ctx, Recipe, Rtx};

#[derive(Clone, Serialize)]
struct LangPage<'r> {
    lang: Option<&'r str>,
    link: String,
}

pub(crate) fn write_recipes(ctx: &mut Ctx, rtx: &Rtx) {
    for recipe in rtx.recipes {
        let langs = rtx
            .recipes
            .iter()
            .filter(|r| r.short == recipe.short && r.lang != recipe.lang)
            .map(|r| LangPage {
                lang: r.lang.as_deref(),
                link: r.stem.to_string() + ".html",
            })
            .collect::<Vec<_>>();
        let index = if rtx.langs.len() < 2 {
            "index.html".to_string()
        } else {
            index_for_lang(recipe.lang.as_deref())
        };
        if let Err(err) = write_recipe(ctx, recipe, rtx.default_lang, &langs, &index)
            .with_context(|| format!("Skipping writing recipe {}", recipe.title))
        {
            ctx.print_error(err);
        }
    }
}

fn write_recipe(
    ctx: &Ctx,
    recipe: &Recipe,
    default_lang: &str,
    langs: &[LangPage],
    index: &str,
) -> Result<()> {
    let html = render(
        &ctx.reg,
        "recipe",
        &json!({
            "recipe": recipe.recipe,
            "title": recipe.title,
            "ctx": template_ctx(ctx),
            "index": index,
            "lang": recipe.lang.as_deref().unwrap_or(default_lang),
            "langs": langs,
        }),
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

pub(crate) fn write_indices(ctx: &mut Ctx<'_>, rtx: &Rtx) {
    if rtx.langs.len() < 2 {
        let lang = rtx.langs.iter().cloned().next().flatten();
        if let Err(err) = create_index(ctx, rtx.recipes, lang, Default::default(), false) {
            ctx.print_error(err);
        }
    } else {
        let langs = rtx
            .langs
            .iter()
            .filter_map(|&l| l)
            .map(|l| {
                (
                    l,
                    LangPage {
                        lang: Some(l),
                        link: index_for_lang(Some(l)),
                    },
                )
            })
            .collect::<Vec<_>>();
        for &(lang, _) in langs.iter() {
            let langs = langs
                .iter()
                .map(|(_, lang)| lang)
                .filter(|l| l.lang != Some(lang))
                .collect::<Vec<_>>();
            if let Err(err) = create_index(ctx, rtx.recipes, Some(lang), &langs, true) {
                ctx.print_error(err);
            }
        }
        let langs = langs.into_iter().map(|(_, lang)| lang).collect::<Vec<_>>();
        if let Err(err) = write_lang_select(ctx, rtx.recipes, &langs) {
            ctx.print_error(err);
        }
    }
}

fn create_index(
    ctx: &Ctx,
    recipes: &[Recipe],
    lang: Option<&str>,
    langs: &[&LangPage],
    localized: bool,
) -> Result<()> {
    let mut this_lang = Vec::new();
    let mut other_lang = Vec::new();
    for recipe in recipes {
        if recipe.lang.as_deref() == lang {
            this_lang.push(recipe);
        } else {
            other_lang.push(recipe);
        }
    }

    let name = index_for_lang(lang.filter(|_| localized));
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(ctx.dest.join(&name))
        .with_context(|| format!("failed to create {name} file"))?;

    file.write_all(
        render(
            &ctx.reg,
            "index",
            &json!({
                "ctx": template_ctx(ctx),
                "this_lang": &this_lang,
                "other_lang": &other_lang,
                "index": "index.html",
                "langs": langs,
                "lang": lang,
            }),
        )?
        .as_bytes(),
    )
    .context("failed to write index.html file")?;

    Ok(())
}

fn write_lang_select(ctx: &mut Ctx<'_>, recipes: &[Recipe], langs: &[LangPage]) -> Result<()> {
    let recipe_count = recipes
        .iter()
        .map(|r| &r.short)
        .collect::<HashSet<_>>()
        .len();

    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(ctx.dest.join("index.html"))
        .context("failed to create index.html")?;

    file.write_all(
        render(
            &ctx.reg,
            "lang",
            &json!({
                "ctx": template_ctx(ctx),
                "recipes": recipes,
                "langs": langs,
                "index": "index.html",
                "recipe_count": recipe_count,
            }),
        )?
        .as_bytes(),
    )
    .context("failed to write index.html file")?;

    Ok(())
}

fn index_for_lang(lang: Option<&str>) -> String {
    let mut name = "index".to_string();
    if let Some(lang) = lang {
        name += ".";
        name += lang;
    }
    name += ".html";
    name
}

fn template_ctx(ctx: &Ctx) -> serde_json::Value {
    json!({
        "links": ctx.links,
        "footer": ctx.footer,
        "title": ctx.title.as_deref().unwrap_or("Recipes"),
        "custom_title": ctx.title.is_some(),
        "version": env!("CARGO_PKG_VERSION"),
    })
}
