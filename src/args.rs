use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use crate::utils::Link;

/// A generator for a static recipe website.
///
/// The exit code is 0 if successful, 1 if a fatal error occurred, and 2 if
/// generation finished with errors.
#[derive(Parser)]
#[command(version)]
pub(crate) struct Args {
    /// Directory with recipes in Markdown format.
    pub(crate) source: PathBuf,
    /// Directory to write generated website to.
    pub(crate) destination: PathBuf,
    /// Add custom website title.
    #[arg(short, long)]
    pub(crate) title: Option<String>,
    /// Add link to footer.
    ///
    /// Values are specified in format "label=href" with label being the shown
    /// text and href being the link.
    /// Can be specified multiple times.
    #[arg(short, long, value_parser=parse_link, value_name="LABEL>=<HREF")]
    pub(crate) link: Vec<Link>,
    /// Add plain text to footer.
    #[arg(short, long)]
    pub(crate) footer: Option<String>,
}

/// Parse link of format `label=href`
fn parse_link(arg: &str) -> Result<Link> {
    let parts = arg
        .split_once('=')
        .with_context(|| format!(r#"link argument "{arg}" does not contain a '='"#))?;
    Ok(Link {
        label: parts.0.to_string(),
        href: parts.1.to_string(),
    })
}
