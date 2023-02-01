use clap::{Parser, Subcommand};

use rsync_rust::handle_signature_command;
use rsync_rust::read_file;

#[derive(Parser)]
struct Arguments {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    // TODO: there must be a way to get Paths here already?
    Signature {
        filename: String,
        output_filename: String,
    },
    Delta {
        signature_filename: String,
        desired_filename: String,
        delta_filename: String,
    },
    Patch,
}

fn main() {
    color_eyre::install().expect("Could not install color_eyre");

    let args = Arguments::parse();

    match args.command {
        Commands::Signature { filename, output_filename } => {
            if let Ok(file_bytes) = rsync_rust::read_file(filename.clone()) {
                handle_signature_command(file_bytes, &output_filename);
            } else {
                println!("Unable to read file: {filename}\n\
                          Are you sure the path provided is correct?");
            }
        }
        Commands::Delta { signature_filename, desired_filename, delta_filename } => println!("Delta"),
        Commands::Patch => println!("Patch"),
    }
}

