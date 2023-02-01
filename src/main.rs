use clap::{Parser, Subcommand};
use color_eyre::eyre::Context;

use rsync_rust::handle_signature_command;

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

    // TODO: this will be changed to be either provided by the caller or read from a config file
    let global_chunk_size = 10;

    match args.command {
        Commands::Signature { filename, output_filename } => {
            match rsync_rust::read_file(filename.clone()) {
                Ok(file_bytes) => {
                    let bytes = handle_signature_command(file_bytes, global_chunk_size);

                    rsync_rust::write_to_file(output_filename, bytes).wrap_err("Unable to write to file").unwrap();
                }
                Err(error) => {
                    println!("Unable to read file: {filename}\n\
                          Are you sure the path provided is correct?\n\
                          Error: {error}");
                }
            }
        }
        Commands::Delta { signature_filename, desired_filename, delta_filename } => println!("Delta"),
        Commands::Patch => println!("Patch"),
    }
}

