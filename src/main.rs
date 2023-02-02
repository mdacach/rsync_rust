use std::process::exit;

use clap::{Parser, Subcommand};
use color_eyre::eyre::Context;

use rsync_rust::{handle_delta_command, handle_signature_command};

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
                    let signature = handle_signature_command(file_bytes, global_chunk_size);

                    rsync_rust::write_to_file(output_filename, signature.into()).wrap_err("Unable to write to file").unwrap();
                }
                Err(error) => {
                    println!("Unable to read file: {filename}\n\
                          Are you sure the path provided is correct?\n\
                          Error: {error}");
                    exit(1);
                }
            }
        }
        Commands::Delta { signature_filename, desired_filename, delta_filename } => {
            let signature_file_bytes = match rsync_rust::read_file(signature_filename.clone()) {
                Ok(bytes) => bytes,
                Err(error) => {
                    println!("Unable to read file: {signature_filename}\n\
                          Are you sure the path provided is correct?\n\
                          Error: {error}");
                    exit(1);
                }
            };

            let our_file_bytes = match rsync_rust::read_file(desired_filename.clone()) {
                Ok(bytes) => bytes,
                Err(error) => {
                    println!("Unable to read file: {desired_filename}\n\
                          Are you sure the path provided is correct?\n\
                          Error: {error}");
                    exit(1);
                }
            };

            let delta = handle_delta_command(signature_file_bytes, our_file_bytes, global_chunk_size);
            rsync_rust::write_to_file(delta_filename, delta.into()).wrap_err("Unable to write to file").unwrap();
        }
        Commands::Patch => println!("Patch"),
    }
}

