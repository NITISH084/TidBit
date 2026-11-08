use anyhow::{Result, bail};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/**
 * Represents the usage of a file in terms of its path, size, and allocated size.
 */
#[derive(Debug, Serialize)]
pub struct FileUsage {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub allocated_size_bytes: u64,
}

/**
 * Represents the usage of a directory, including its path, total size, allocated size, and the files it contains.
 */
#[derive(Debug, Serialize)]
pub struct Directoryusage {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub allocated_size_bytes: u64,
}

/**
 * Represents an error that occurred while scanning a directory
**/
#[derive(Debug, Serialize)]
pub struct ScanError {
    pub path: PathBuf,
    pub message: String,
}

/**
 * Represents the report generated after scanning a directory,
 *  including the root path, total sizes, counts of files and directories scanned,
 *largest files and directories, and any errors encountered.
 */
#[derive(Debug, Serialize)]
pub struct ScanReport {
    pub scan_root: PathBuf,
    pub total_size_bytes: u64,
    pub total_allocated_size_bytes: u64,
    pub files_scanned: u64,
    pub directories_scanned: u64,
    pub largest_files: Vec<FileUsage>,
    pub largest_directories: Vec<Directoryusage>,
    pub errors: Vec<ScanError>,
}

/**
 * Gets the allocated size of a file/disk based on its metadata.
 * For Unix platform only.
 */
#[cfg(unix)]
fn get_allocated_size(metadata: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    metadata.blocks() * 512
}

/*
* Fallback for non-Unix platforms, returns the file size as the allocated size.
*/
#[cfg(not(unix))]
fn get_allocated_size(metadata: &std::fs::Metadata) -> u64 {
    metadata.len()
}

pub fn scan_directory(root: impl AsRef<Path>, top_n: usize) -> Result<ScanReport> {
    let root = root.as_ref().to_path_buf();
    if !root.exists() || !root.is_dir() {
        bail!(
            "The provided path is not a valid directory: {:?}",
            root.display()
        )
    }

    let mut total_size_bytes = 0;
    let mut total_allocated_size_bytes = 0;

    let mut files_scanned = 0;
    let mut directories_scanned = 0;

    let mut largest_files: Vec<FileUsage> = Vec::new();
    let mut directory_sizes: HashMap<PathBuf, (u64, u64)> = HashMap::new();

    let mut errors: Vec<ScanError> = Vec::new();

    for entry_result in WalkDir::new(&root).follow_links(false) {
        let entry = match entry_result {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(ScanError {
                    path: error.path().unwrap_or(&root).to_path_buf(),
                    message: error.to_string(),
                });
                continue;
            }
        };

        let path = entry.path();

        if entry.file_type().is_dir() {
            directories_scanned += 1;

            // If the directory is empty, we still want to record its size as 0
            directory_sizes.entry(path.to_path_buf()).or_insert((0, 0));

            continue;
        }

        // Only regular files contribute to file storage totals.
        // Symlinks and other special file types are not counted in the total size or allocated size.
        if !entry.file_type().is_file() {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                errors.push(ScanError {
                    path: path.to_path_buf(),
                    message: error.to_string(),
                });
                continue;
            }
        };

        let size = metadata.len();
        let allocated_size = get_allocated_size(&metadata);

        total_size_bytes += size;
        total_allocated_size_bytes += allocated_size;

        files_scanned += 1;

        let file_usage = FileUsage {
            path: path.to_path_buf(),
            size_bytes: size,
            allocated_size_bytes: allocated_size,
        };

        largest_files.push(file_usage);

        // Add the file's size to its parent directory's total size.
        let mut current = path.parent();

        while let Some(directory) = current {
            if !directory.starts_with(&root) {
                break;
            }

            let usage = directory_sizes
                .entry(directory.to_path_buf())
                .or_insert((0, 0));

            usage.0 += size;
            usage.1 += allocated_size;

            if directory == root {
                break;
            }

            current = directory.parent();
        }
    }

    // Sort the largest files in descending order based on their size.
    largest_files.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    largest_files.truncate(top_n);

    // Convert the directory aggregrates into a List.
    let mut largest_directories: Vec<Directoryusage> = directory_sizes
        .into_iter()
        .map(
            |(path, (size_bytes, allocated_size_bytes))| Directoryusage {
                path,
                size_bytes,
                allocated_size_bytes,
            },
        )
        .collect();

    largest_directories.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    largest_directories.truncate(top_n);

    Ok(ScanReport {
        scan_root: root,
        total_size_bytes,
        total_allocated_size_bytes,
        files_scanned,
        directories_scanned,
        largest_files,
        largest_directories,
        errors,
    })
}
