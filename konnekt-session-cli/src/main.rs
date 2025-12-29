use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "konnekt-cli")]
#[command(
    version,
    about = "Konnekt Session CLI - Schema generation and development tools"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {}

fn main() {
    let cli = Cli::parse();
}
