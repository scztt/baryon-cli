// @TODO remove later
#![allow(dead_code)]

mod actions;
mod cli;
mod core;
mod specs;

use cli::cli;
use miette::Result;

#[tokio::main]
async fn main() -> Result<()> {
    cli().await?;
    Ok(())
}
