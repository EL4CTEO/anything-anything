//! `{{command_synopsis}}`: {{command_about}}
//!
//! Yours: anything-anything never overwrites this file.

use super::Output;
use super::args::{{command_pascal}}Args;
use anyhow::Result;

pub fn run(args: &{{command_pascal}}Args) -> Result<Output> {
    // ANYTHING:BUILD replace this with the real {{command}}
    anyhow::bail!("`{{command}}` isn't built yet ({args:?}): run anything-anything build")
}
