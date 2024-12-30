mod args;
mod files;
mod l10n;
mod parsing;
mod utils;
mod writing;

use std::{
    fs::{create_dir, read_dir, remove_dir_all, DirEntry},
    path::Path,
    process::ExitCode,
};

use anyhow::{bail, Context, Result};
use args::Args;
use clap::Parser;
use files::*;
use handlebars::Handlebars;
use l10n::L10nHelper;
use parsing::*;
use utils::*;
use writing::{write_indices, write_recipes};

fn main() -> Result<ExitCode> {
    let args = Args::parse();
    let mut reg = handlebars_registry(args.templates.as_deref())?;
    reg.register_helper("l10n", Box::new(L10nHelper::new(args.lang.clone())?));
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
        remove_dest(&ctx.dest)?;
    }
    create_dest(&ctx.dest)?;
    create_static(&mut ctx);

    // Copy source files after creating static files to allow overriding them.
    let mut recipes = process_source_dir(&mut ctx)?;
    recipes.sort_unstable();
    let rtx = Rtx::new(&recipes, &args.lang);

    write_recipes(&mut ctx, &rtx);
    write_indices(&mut ctx, &rtx);

    Ok(if ctx.any_error {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    })
}

fn remove_dest(path: &Path) -> Result<()> {
    remove_dir_all(path).with_context(|| {
        format!(
            "Failed to remove destination directory {}",
            path.to_string_lossy()
        )
    })
}

fn create_dest(dest: &Path) -> Result<()> {
    create_dir(dest).with_context(|| {
        format!(
            "Failed to create destination directory {}",
            dest.to_string_lossy()
        )
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
