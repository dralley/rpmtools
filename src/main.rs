// Copyright (c) 2025 Daniel Alley
// 
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::error::Error;
use std::path::PathBuf;

use argh;
use argh::FromArgs;

use rpmtools;

#[derive(FromArgs, PartialEq, Debug)]
/// Top-level command.
struct TopLevel {
    #[argh(subcommand)]
    subcommands: Subcommands,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum Subcommands {
    Split(SplitArgs),
    Extract(ExtractArgs),
    List(ListArgs),
    Tree(TreeArgs),
}

#[derive(FromArgs, PartialEq, Debug)]
/// Split subcommand.
#[argh(subcommand, name = "split")]
struct SplitArgs {
    #[argh(positional)]
    /// the path to the RPM taken as input
    input: PathBuf,
    #[argh(option)]
    /// where to dump the package components
    destination: Option<PathBuf>,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Extract subcommand.
#[argh(subcommand, name = "extract")]
struct ExtractArgs {
    #[argh(positional)]
    /// the path to the RPM taken as input
    input: PathBuf,
    #[argh(option)]
    /// where to dump the package payload contents
    destination: Option<PathBuf>,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Take a look at the list of files in the package payload
#[argh(subcommand, name = "list")]
struct ListArgs {
    #[argh(positional)]
    /// the path to the RPM taken as input
    input: PathBuf,
}

#[derive(FromArgs, PartialEq, Debug)]

/// Take a look at the list of files in the package payload formatted as a "tree"
#[argh(subcommand, name = "tree")]
struct TreeArgs {
    #[argh(positional)]
    /// the path to the RPM taken as input
    input: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: TopLevel = argh::from_env();

    match args.subcommands {
        Subcommands::Split(args) => {
            rpmtools::split_package_into_components(&args.input, args.destination)?
        }
        Subcommands::Extract(args) => {
            rpmtools::extract_package_payload(&args.input, args.destination)?
        }
        Subcommands::List(args) => rpmtools::print_package_file_list(&args.input)?,
        Subcommands::Tree(args) => rpmtools::print_package_file_tree(&args.input)?,
        // TODO:
        // * print package tags and such i.e. rpmdump
        // * rpmsort
        // * recompress package payload
        // * canonicalize RPM (sort tags, etc.)
    }

    Ok(())
}
