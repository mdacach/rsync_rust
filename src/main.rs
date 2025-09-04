//! rsync main idea:
//! User A has a initial file, let's call it `basis_file`.
//! We (User B) have made some changes to this file, and have our own version of it, `updated_file`.
//! Now we want to send our changes to user A, so they can update their `basis_file` to be equal
//! to our `updated_file`.
//!
//! One way of accomplishing that is sending the `updated_file` directly.
//! 1 - User B sends `updated_file`
//! 2 - User A replaces `basis_file` with `updated_file`
//!
//! This of course works, but we are not leveraging the facts that both `basis_file` and `updated_file`
//! are bound to have mostly the same content. (Picture `basis_file` as a Git repository,
//! and `updated_file` as the repository after you've made some commits).
//!
//! The rsync algorithm:
//! 1 - User A computes a `signature` for `basis_file`.
//!         This `signature` "represents" the contents of `basis_file`, approximately, and is much smaller.
//!
//! 2 - User A sends the `signature` to User B.
//!
//! 3 - User B uses `signature` to compute `delta` from `updated_file` to `basis_file`.
//!         This `delta` has exactly what needs to change from `basis_file` to become `updated_file`.
//!
//! 4 - User B sends `delta` to User A.
//!         In an approximate way, the `delta` encompasses only what needs to be changed between the two files
//!         which is generally much smaller than the whole file.
//!
//! 5 - User A uses `delta` to update `basis_file`.
//!
//! In the end, we have sent two files throughout the algorithm: `signature` and `delta`.
//! As long as `size(signature)` + `size(delta)` is smaller than `size(updated_file)` we have made
//! improvements regarding network resources.
//!
//! Note that we have traded computation time for memory.
//! We are sending smaller files through the network, but both User A and User B need to
//! compute information based on that.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use color_eyre::eyre::Context;

use rsync_rust::domain::delta::compute_delta_to_our_file;
use rsync_rust::domain::patch::apply_delta;
use rsync_rust::domain::signature::compute_signature;
use rsync_rust::io_utils;

#[derive(Parser)]
struct Arguments {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
// TODO (Clap): Investigate if possible to validate the file formats within Clap
//              e.g: `signature_filename` needs to be convertible to FileSignature
enum Commands {
    Signature {
        // The basis file to compute Signature from.
        basis_filename: PathBuf,
        // Where to save the Signature file.
        signature_output_filename: PathBuf,
        // Size for each block.
        #[arg(short, long, default_value_t = 10)]
        chunk_size: usize,
    },
    Delta {
        // Signature file computed by `Signature` command.
        signature_filename: PathBuf,
        // File to compute `Delta` from `Signature`.
        updated_filename: PathBuf,
        // Where to save the `Delta` file.
        delta_filename: PathBuf,
        // Size for each block.
        #[arg(short, long, default_value_t = 10)]
        chunk_size: usize,
    },
    Patch {
        // File to apply changes.
        basis_filename: PathBuf,
        // Delta file computed by `Delta` command.
        delta_filename: PathBuf,
        // Where to save the updated file.
        recreated_filename: PathBuf,
        #[arg(short, long, default_value_t = 10)]
        chunk_size: usize, // Size for each block.
    },
}

fn main() -> color_eyre::Result<(), color_eyre::Report> {
    // For prettier errors.
    color_eyre::install().expect("Could not install color_eyre");

    let args = Arguments::parse();

    match args.command {
        Commands::Signature {
            basis_filename,
            signature_output_filename,
            chunk_size,
        } => handle_signature_command(basis_filename, signature_output_filename, chunk_size),
        Commands::Delta {
            signature_filename,
            updated_filename,
            delta_filename,
            chunk_size,
        } => handle_delta_command(
            signature_filename,
            updated_filename,
            delta_filename,
            chunk_size,
        ),
        Commands::Patch {
            basis_filename,
            delta_filename,
            recreated_filename,
            chunk_size,
        } => handle_patch_command(
            basis_filename,
            delta_filename,
            recreated_filename,
            chunk_size,
        ),
    }
}

fn handle_signature_command(
    basis_filename: PathBuf,
    signature_output_filename: PathBuf,
    chunk_size: usize,
) -> color_eyre::Result<(), color_eyre::Report> {
    let basis_file_bytes = io_utils::attempt_to_read_file(basis_filename)
        .context("Error while reading Basis file provided as argument for `signature` command")?;

    let signature = compute_signature(basis_file_bytes, chunk_size);

    let signature_bytes = signature.try_into()?;
    io_utils::write_to_file(&signature_output_filename, signature_bytes).wrap_err(format!(
        "Unable to write to file: {}",
        &signature_output_filename.display()
    ))
}

fn handle_delta_command(
    signature_filename: PathBuf,
    updated_filename: PathBuf,
    delta_filename: PathBuf,
    chunk_size: usize,
) -> color_eyre::Result<(), color_eyre::Report> {
    let signature_file_bytes = io_utils::attempt_to_read_file(&signature_filename)
        .context("Error while reading Signature file provided as argument to `delta` command")?;
    let updated_file_bytes = io_utils::attempt_to_read_file(updated_filename)
        .context("Error while reading Updated file provided as argument to `delta` command")?;

    let signature = signature_file_bytes.try_into().context(format!(
        r#"Signature file path provided was "{}"."#,
        &signature_filename.display()
    ))?;
    let delta = compute_delta_to_our_file(signature, updated_file_bytes, chunk_size);

    let delta_bytes = delta.try_into()?;
    io_utils::write_to_file(&delta_filename, delta_bytes).wrap_err(format!(
        "Unable to write to file: {}",
        &delta_filename.display()
    ))
}

fn handle_patch_command(
    basis_filename: PathBuf,
    delta_filename: PathBuf,
    recreated_filename: PathBuf,
    chunk_size: usize,
) -> color_eyre::Result<(), color_eyre::Report> {
    let basis_file_bytes = io_utils::attempt_to_read_file(basis_filename)
        .context("Error while reading Basis file provided as argument to `patch` command")?;
    let delta_file_bytes = io_utils::attempt_to_read_file(&delta_filename)
        .context("Error while reading Delta file provided as argument to `patch` command")?;

    let delta = delta_file_bytes.try_into().context(format!(
        r#"Delta file path provided was "{}"."#,
        &delta_filename.display()
    ))?;
    let recreated = apply_delta(basis_file_bytes, delta, chunk_size);

    io_utils::write_to_file(&recreated_filename, recreated).wrap_err(format!(
        "Unable to write to file: {}",
        &recreated_filename.display()
    ))
}
