/// Utility functions for handling dates and data source names.

/// This function extracts the file name from a given path.
/// If the path does not contain a valid file name, it returns the original path as a string.
pub fn data_source_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}
