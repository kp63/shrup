//! File path resolution and include processing

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Context;

use crate::error::{PreprocessorError, Result};
use crate::parser::IncludeDirective;

/// Preprocessor configuration
#[derive(Debug, Clone)]
pub struct ProcessingConfig {
    /// Add debug comments to output
    pub debug_mode: bool,
    /// Max include depth to prevent recursion
    pub max_include_depth: usize,
    /// Base directory for path resolution
    pub base_directory: PathBuf,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            debug_mode: false,
            max_include_depth: 100,
            base_directory: PathBuf::from("."),
        }
    }
}

/// Processing state context
#[derive(Debug)]
pub struct ProcessingContext {
    /// Files already visited (circular dependency detection)
    visited_files: HashSet<PathBuf>,
    /// Stack of processing files (for error reporting)
    include_stack: Vec<PathBuf>,
    /// Config
    config: ProcessingConfig,
}

impl ProcessingContext {
    /// Create processing context
    pub fn new(config: ProcessingConfig) -> Self {
        Self {
            visited_files: HashSet::new(),
            include_stack: Vec::new(),
            config,
        }
    }
    
    /// Get current include depth
    pub fn current_depth(&self) -> usize {
        self.include_stack.len()
    }
    
    /// Check if max depth exceeded
    pub fn check_max_depth(&self, file_path: &Path) -> Result<()> {
        if self.current_depth() >= self.config.max_include_depth {
            return Err(PreprocessorError::MaxDepthExceeded {
                path: file_path.to_path_buf(),
                max_depth: self.config.max_include_depth,
            }.into());
        }
        Ok(())
    }
    
    /// Check circular dependencies
    pub fn check_circular_dependency(&self, file_path: &Path) -> Result<()> {
        let canonical_path = self.canonicalize_path(file_path)?;
        
        if self.visited_files.contains(&canonical_path) {
            let stack_str = self.include_stack
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(" -> ");
                
            return Err(PreprocessorError::CircularDependency {
                path: canonical_path,
                stack: stack_str,
            }.into());
        }
        
        Ok(())
    }
    
    /// Enter file (push to stack and visited set)
    pub fn enter_file(&mut self, file_path: &Path) -> Result<()> {
        let canonical_path = self.canonicalize_path(file_path)?;
        
        self.check_max_depth(&canonical_path)?;
        self.check_circular_dependency(&canonical_path)?;
        
        self.visited_files.insert(canonical_path.clone());
        self.include_stack.push(canonical_path);
        
        Ok(())
    }
    
    /// Exit file (pop from stack, keep in visited set)
    pub fn exit_file(&mut self) {
        self.include_stack.pop();
    }
    
    /// Get config
    pub fn config(&self) -> &ProcessingConfig {
        &self.config
    }
    
    /// Canonicalize path for consistent comparison
    fn canonicalize_path(&self, path: &Path) -> Result<PathBuf> {
        path.canonicalize()
            .with_context(|| format!("Failed to canonicalize path: {}", path.display()))
    }
}

/// File resolver for include directives
pub struct FileResolver;

impl FileResolver {
    /// Resolve file path for include directive
    pub fn resolve_include_path(
        directive: &IncludeDirective,
        config: &ProcessingConfig,
    ) -> Result<PathBuf> {
        let include_path = Path::new(&directive.file_path);
        
        // Absolute path: resolve relative to base directory
        let resolved_path = if include_path.is_absolute() {
            config.base_directory.join(include_path.strip_prefix("/").unwrap_or(include_path))
        } else {
            // Relative path: resolve relative to source file directory
            let source_dir = directive.source_file
                .parent()
                .unwrap_or_else(|| Path::new("."));
            source_dir.join(include_path)
        };
        
        // Check file exists
        if !resolved_path.exists() {
            return Err(PreprocessorError::FileNotFound {
                path: resolved_path,
            }.into());
        }
        
        // Check it's a file, not directory
        if !resolved_path.is_file() {
            return Err(PreprocessorError::FileNotFound {
                path: resolved_path,
            }.into());
        }
        
        Ok(resolved_path)
    }
    
    /// Read file content with error handling
    pub fn read_file_content(file_path: &Path) -> Result<String> {
        fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))
            .map_err(|e| {
                // Check specific error types
                if let Some(io_error) = e.downcast_ref::<std::io::Error>() {
                    match io_error.kind() {
                        std::io::ErrorKind::PermissionDenied => {
                            return PreprocessorError::PermissionDenied {
                                path: file_path.to_path_buf(),
                            }.into();
                        }
                        std::io::ErrorKind::NotFound => {
                            return PreprocessorError::FileNotFound {
                                path: file_path.to_path_buf(),
                            }.into();
                        }
                        _ => {}
                    }
                }
                e
            })
    }
    
    /// Generate debug comment for include
    pub fn generate_include_comment(file_path: &Path, is_start: bool) -> String {
        let display_path = file_path.display();
        if is_start {
            format!("# --- Included from {} ---", display_path)
        } else {
            format!("# --- End of {} ---", display_path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[test]
    fn test_processing_context_max_depth() {
        let mut config = ProcessingConfig::default();
        config.max_include_depth = 2;
        
        let mut context = ProcessingContext::new(config);
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.sh");
        fs::write(&file_path, "content").unwrap();
        
        // Should succeed for first two files
        assert!(context.enter_file(&file_path).is_ok());
        assert!(context.enter_file(&file_path).is_err()); // Circular dependency
    }
    
    #[test]
    fn test_resolve_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        let source_file = temp_dir.path().join("main.sh");
        let include_file = temp_dir.path().join("utils.sh");
        
        fs::write(&source_file, "").unwrap();
        fs::write(&include_file, "").unwrap();
        
        let directive = IncludeDirective::new(
            1,
            "utils.sh".to_string(),
            source_file,
            crate::parser::IncludeQuoteType::DoubleQuotes,
        );
        
        let config = ProcessingConfig {
            base_directory: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let resolved = FileResolver::resolve_include_path(&directive, &config).unwrap();
        assert_eq!(resolved, include_file);
    }
}