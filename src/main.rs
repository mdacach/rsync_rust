//! rsync main idea:
//! User A has a initial file, let's call it `file1`.
//! We (User B) have made some changes to this file, and have our own version of it, `file2`.
//! Now we want to send our changes to user A, so they can update their `file1` to be equal
//! to our `file2`.
//!
//! One way of accomplishing that is sending the `file2` directly.
//! 1 - User B sends `file2`
//! 2 - User A replaces `file1` with `file2`
//!
//! This of course works, but we are not leveraging the facts that both `file1` and `file2`
//! are bound to have mostly the same content. (Picture `file1` as a Git repository,
//! and `file2` as the repository after you've made some commits).
//!
//! The rsync algorithm:
//! 1 - User A computes a `signature` for `file1`.
//!         This `signature` "represents" the contents of `file1`, approximately, and is much smaller.
//!
//! 2 - User A sends the `signature` to User B.
//!
//! 3 - User B uses `signature` to compute `delta` from `file2` to `file1`.
//!         This `delta` has exactly what needs to change from `file1` to become `file2`.
//!
//! 4 - User B sends `delta` to User A.
//!         In an approximate way, the `delta` encompasses only what needs to be changed between the two files
//!         which is generally much smaller than the whole file.
//!
//! 5 - User A uses `delta` to update `file1`.
//!
//! In the end, we have sent two files throughout the algorithm: `signature` and `delta`.
//! As long as `size(signature)` + `size(delta)` is smaller than `size(file2)` we have made
//! improvements regarding network resources.
//!
//! Note that we have traded computation time for memory.
//! We are sending smaller files through the network, but both User A and User B need to
//! compute information based on that.

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
// TODO (Clap): Some way to use Path here instead of String?
//              Investigate if possible to validate the file formats within Clap
//              e.g: `signature_filename` needs to be convertible to FileSignature
enum Commands {
    Signature {
        filename: String,
        // The basis file to compute Signature from.
        output_filename: String,
        // Where to save the Signature file.
        #[arg(short, long, default_value_t = 10)]
        chunk_size: usize, // Size for each block.
    },
    Delta {
        signature_filename: String,
        // Signature file computed by `Signature` command.
        our_file_filename: String,
        // File to compute `Delta` from `Signature`.
        delta_filename: String,
        // Where to save the `Delta` file.
        #[arg(short, long, default_value_t = 10)]
        chunk_size: usize, // Size for each block.
    },
    Patch {
        basis_filename: String,
        // File to apply changes.
        delta_filename: String,
        // Delta file computed by `Delta` command.
        recreated_filename: String,
        // Where to save the updated file.
        #[arg(short, long, default_value_t = 10)]
        chunk_size: usize, // Size for each block.
    },
}

fn main() {
    // For prettier errors.
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
