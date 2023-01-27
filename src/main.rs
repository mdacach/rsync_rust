use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Arguments {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Signature {
        file: String,
        output_file: String,
    },
    Delta,
    Patch,
}

fn main() {
    let args = Arguments::parse();

    match args.command {
        Commands::Signature { file, output_file } => {},
        Commands::Delta => println!("Delta"),
        Commands::Patch => println!("Patch"),
    }
}

