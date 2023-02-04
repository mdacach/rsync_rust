use std::process::exit;

use clap::{Parser, Subcommand};
use color_eyre::eyre::Context;

use rsync_rust::delta::{compute_delta_to_our_file, Delta};
use rsync_rust::io_utils;
use rsync_rust::patch::apply_delta;
use rsync_rust::signature::compute_signature;

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
        #[arg(short, long, default_value_t = 10)]
        chunk_size: usize,
    },
    Delta {
        signature_filename: String,
        our_file_filename: String,
        delta_filename: String,
        #[arg(short, long, default_value_t = 10)]
        chunk_size: usize,
    },
    Patch {
        basis_filename: String,
        delta_filename: String,
        recreated_filename: String,
        #[arg(short, long, default_value_t = 10)]
        chunk_size: usize,
    },
}

fn main() {
    color_eyre::install().expect("Could not install color_eyre");

    let args = Arguments::parse();

    match args.command {
        Commands::Signature {
            filename,
            output_filename,
            chunk_size,
        } => {
            handle_signature_command(filename, output_filename, chunk_size);
        }
        Commands::Delta {
            signature_filename,
            our_file_filename,
            delta_filename,
            chunk_size,
        } => {
            handle_delta_command(
                signature_filename,
                our_file_filename,
                delta_filename,
                chunk_size,
            );
        }
        Commands::Patch {
            basis_filename,
            delta_filename,
            recreated_filename,
            chunk_size,
        } => {
            handle_patch_command(
                basis_filename,
                delta_filename,
                recreated_filename,
                chunk_size,
            );
        }
    }
}

fn handle_signature_command(filename: String, output_filename: String, chunk_size: usize) {
    match io_utils::read_file(filename.clone()) {
        Ok(file_bytes) => {
            let signature = compute_signature(file_bytes, chunk_size);

            io_utils::write_to_file(output_filename, signature.into())
                .wrap_err("Unable to write to file")
                .unwrap();
        }
        Err(error) => {
            println!(
                "Unable to read file: {filename}\n\
                          Are you sure the path provided is correct?\n\
                          Error: {error}"
            );
            exit(1);
        }
    }
}

fn handle_delta_command(
    signature_filename: String,
    our_file_filename: String,
    delta_filename: String,
    chunk_size: usize,
) {
    let signature_file_bytes = match io_utils::read_file(signature_filename.clone()) {
        Ok(bytes) => bytes,
        Err(error) => {
            println!(
                "Unable to read file: {signature_filename}\n\
                          Are you sure the path provided is correct?\n\
                          Error: {error}"
            );
            exit(1);
        }
    };

    let our_file_bytes = match io_utils::read_file(our_file_filename.clone()) {
        Ok(bytes) => bytes,
        Err(error) => {
            println!(
                "Unable to read file: {our_file_filename}\n\
                          Are you sure the path provided is correct?\n\
                          Error: {error}"
            );
            exit(1);
        }
    };

    let delta = compute_delta_to_our_file(signature_file_bytes.into(), our_file_bytes, chunk_size);
    io_utils::write_to_file(delta_filename, delta.into())
        .wrap_err("Unable to write to file")
        .unwrap();
}

fn handle_patch_command(
    basis_filename: String,
    delta_filename: String,
    recreated_filename: String,
    chunk_size: usize,
) {
    let basis_file_bytes = match io_utils::read_file(basis_filename.clone()) {
        Ok(bytes) => bytes,
        Err(error) => {
            println!(
                "Unable to read file: {basis_filename}\n\
                          Are you sure the path provided is correct?\n\
                          Error: {error}"
            );
            exit(1);
        }
    };

    let delta_file_bytes = match io_utils::read_file(delta_filename.clone()) {
        Ok(bytes) => bytes,
        Err(error) => {
            println!(
                "Unable to read file: {delta_filename}\n\
                          Are you sure the path provided is correct?\n\
                          Error: {error}"
            );
            exit(1);
        }
    };

    let delta: Delta = delta_file_bytes.into();
    let recreated = apply_delta(basis_file_bytes, delta, chunk_size);
    io_utils::write_to_file(recreated_filename, recreated)
        .wrap_err("Unable to write to file")
        .unwrap();
}
