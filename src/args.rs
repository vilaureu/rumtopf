use std::path::PathBuf;

use clap::Parser;

/// A generator for a static recipe website.
#[derive(Parser)]
#[command(version)]
pub(crate) struct Args {
    /// Directory with recipes in Markdown format.
    pub(crate) source: PathBuf,
    /// Directory to write generated website to.
    pub(crate) destination: PathBuf,
}
