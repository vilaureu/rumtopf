use std::fs::write;

use anyhow::Context;

use crate::utils::Ctx;

include!(concat!(env!("OUT_DIR"), "/files.rs"));

pub(crate) fn create_static(ctx: &mut Ctx) {
    for (name, content) in STATIC {
        if let Err(err) = write(ctx.dest.join(name), content)
            .with_context(|| format!("Failed to write static file {name}"))
        {
            ctx.print_error(err);
        }
    }
}
