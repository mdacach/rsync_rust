use clap::{Parser, Subcommand};

use rsync_rust::handle_signature_command;

#[derive(Parser)]
struct Arguments {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Signature {
        filename: String,
        output_filename: String,
    },
    Delta,
    Patch,
}

fn main() {
    let args = Arguments::parse();

    match args.command {
        Commands::Signature { filename, output_filename } => {
            handle_signature_command(&filename, &output_filename);
        }
        Commands::Delta => println!("Delta"),
        Commands::Patch => println!("Patch"),
    }
}

