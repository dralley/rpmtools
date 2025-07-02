// Copyright (c) 2025 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use rpm;

pub fn split_package_into_components(
    pkg_path: &Path,
    destination: Option<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    let metadata = rpm::PackageMetadata::open(pkg_path)?;
    let offsets = metadata.get_package_segment_offsets();

    let epoch = metadata.get_epoch().unwrap_or(0).to_string();
    let package_nevra = rpm::Nevra::new(
        metadata.get_name()?,
        &epoch,
        metadata.get_version()?,
        metadata.get_release()?,
        metadata.get_arch()?,
    );
    let dest_path = destination
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(package_nevra.to_string()));

    fs::create_dir_all(&dest_path)?;

    let input_bytes = fs::read(pkg_path)?;

    // Helper to write a slice to a file in dest_path
    fn write_section(dest: &Path, name: &str, data: &[u8]) -> Result<(), Box<dyn Error>> {
        let mut file_path = dest.to_path_buf();
        file_path.push(name);
        fs::write(file_path, data)?;
        Ok(())
    }

    // Write lead section
    write_section(
        &dest_path,
        "lead",
        &input_bytes[offsets.lead as usize..offsets.signature_header as usize],
    )?;

    // Write signature header section
    write_section(
        &dest_path,
        "sig_header",
        &input_bytes[offsets.signature_header as usize..offsets.header as usize],
    )?;

    // Write header section
    write_section(
        &dest_path,
        "header",
        &input_bytes[offsets.header as usize..offsets.payload as usize],
    )?;

    // Write payload section
    write_section(
        &dest_path,
        "payload",
        &input_bytes[offsets.payload as usize..],
    )?;

    Ok(())
}

pub fn extract_package_payload(
    pkg_path: &Path,
    destination: Option<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    let package = rpm::Package::open(pkg_path)?;
    let epoch = package.metadata.get_epoch().unwrap_or(0).to_string();
    let package_nevra = rpm::Nevra::new(
        package.metadata.get_name()?,
        &epoch,
        package.metadata.get_version()?,
        package.metadata.get_release()?,
        package.metadata.get_arch()?,
    );
    let dest_path = destination
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(package_nevra.to_string()));

    package.extract(&dest_path)?;

    for f in package.files()? {
        let f = f?;
        println!("{}", f.metadata.path.display());
    }
    Ok(())
}

pub fn print_package_file_list(pkg_path: &Path) -> Result<(), Box<dyn Error>> {
    let package = rpm::Package::open(pkg_path)?;

    let mut paths: Vec<_> = package
        .files()?
        .map(|f| f.map(|file| file.metadata.path.clone()))
        .collect::<Result<_, _>>()?;

    for path in &paths {
        println!("{}", path.display());
    }
    Ok(())
}

pub fn print_package_file_tree(pkg_path: &Path) -> Result<(), Box<dyn Error>> {
    let package = rpm::Package::open(pkg_path)?;

    let mut paths: Vec<_> = package
        .files()?
        .map(|f| f.map(|file| file.metadata.path.clone()))
        .collect::<Result<_, _>>()?;

    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    /// Represents a node in the file tree. It can be a directory (which contains other nodes)
    /// or a file (which is a leaf, represented by a node with no children).
    ///
    /// A struct containing a `Box` is used here to break the recursive type cycle.
    /// A direct type alias like `type TreeNode = BTreeMap<OsString, TreeNode>` is not allowed
    /// in Rust as it would be infinitely recursive. The `Box` provides a layer of indirection
    /// (a pointer to heap-allocated data), giving the `TreeNode` struct a known size at compile time.
    #[derive(Default)]
    struct TreeNode {
        children: BTreeMap<OsString, Box<TreeNode>>,
    }

    /// Takes a slice of `PathBuf`s and prints them in a tree-like format.
    ///
    /// # Arguments
    ///
    /// * `paths` - A slice of `PathBuf` representing the file and directory paths.
    pub fn tree_display(paths: &[PathBuf]) {
        if paths.is_empty() {
            println!(".");
            return;
        }

        // A BTreeMap is used to ensure the entries are sorted alphabetically by name.
        let mut tree = TreeNode::default();

        // Build the tree structure from the flat list of paths.
        for path in paths {
            // Start at the root of the tree for each path.
            let mut current_level = &mut tree.children;
            // Iterate over each component of the path (e.g., "src", "main.rs" in "src/main.rs").
            for component in path.components() {
                // `entry` gets or inserts a node.
                // `.or_default()` creates a default `Box<TreeNode>` if the entry doesn't exist.
                // We then get a mutable reference to the `children` map of that node for the next level.
                current_level = &mut current_level
                    .entry(component.as_os_str().to_owned())
                    .or_default()
                    .children;
            }
        }

        // Print the root directory and start the recursive printing process.
        println!(".");
        print_tree_recursive(&tree, "");
    }
    /// Recursively prints the file tree.
    ///
    /// # Arguments
    ///
    /// * `tree` - The current `TreeNode` (sub-tree) to print.
    /// * `prefix` - The string prefix to use for the current line (e.g., "│   ", "    ").
    fn print_tree_recursive(tree: &TreeNode, prefix: &str) {
        let mut iter = tree.children.iter().peekable();
        while let Some((name, node)) = iter.next() {
            // Check if the current entry is the last one at this level.
            let is_last = iter.peek().is_none();

            // Determine the connector: "└──" for the last item, "├──" for others.
            let connector = if is_last { "└── " } else { "├── " };
            println!("{}{}{}", prefix, connector, name.to_string_lossy());

            // Determine the prefix for the children of this node.
            // If the current node is the last one, the new prefix doesn't need a vertical bar.
            let child_prefix = if is_last { "    " } else { "│   " };
            let new_prefix = format!("{}{}", prefix, child_prefix);

            // If the child node is not empty (i.e., it's a directory with contents), recurse.
            // The `node` is a `Box<TreeNode>`, which auto-dereferences to `&TreeNode` for the call.
            if !node.children.is_empty() {
                print_tree_recursive(node, &new_prefix);
            }
        }
    }
    tree_display(&paths);
    Ok(())
}
