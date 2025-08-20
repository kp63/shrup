//! Preprocessor error types

use std::path::PathBuf;

/// Preprocessor error types
#[derive(Debug, thiserror::Error)]
pub enum PreprocessorError {
    /// File not found
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },
    
    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    /// Circular dependency
    #[error("Circular dependency detected: {path} (include stack: {stack})")]
    CircularDependency {
        path: PathBuf,
        stack: String,
    },
    
    /// Invalid include directive
    #[error("Invalid include directive at line {line_number}: {directive}")]
    InvalidIncludeDirective {
        line_number: usize,
        directive: String,
    },
    
    /// Max include depth exceeded
    #[error("Maximum include depth ({max_depth}) exceeded at: {path}")]
    MaxDepthExceeded { path: PathBuf, max_depth: usize },
    
    /// Permission denied
    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },
}

/// Result type for preprocessor operations
pub type Result<T> = anyhow::Result<T>;