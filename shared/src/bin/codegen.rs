//! Generates the Swift "App" package (value types with bincode serializers)
//! for the SwiftUI shell. Mirrors the crux `examples/counter` codegen bin,
//! Swift-only. Run via `just typegen`.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use crux_core::type_generation::facet::{Config, TypeRegistry};
use shared::Daily;

#[derive(Parser)]
#[command(version, about = "Generate Swift value types for the Daily app")]
struct Args {
    /// Directory the "App" Swift package is generated under.
    #[arg(long, default_value = "apple/generated")]
    output_dir: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let generator = TypeRegistry::new().register_app::<Daily>()?.build()?;
    let config = Config::builder("App", &args.output_dir).build();
    generator.swift(&config)?;

    println!(
        "Swift types written to {}",
        args.output_dir.join("App").display()
    );

    Ok(())
}
