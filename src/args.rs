use std::path::PathBuf;

use clap::Parser;

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
}
