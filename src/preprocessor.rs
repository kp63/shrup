//! Main preprocessor functionality

use std::path::{Path, PathBuf};
use anyhow::Context;

use crate::error::Result;
use crate::parser::{IncludeParser, IncludeDirective};
use crate::resolver::{ProcessingConfig, ProcessingContext, FileResolver};

/// Shell script preprocessor
pub struct ShellPreprocessor {
    config: ProcessingConfig,
}

impl ShellPreprocessor {
    /// Create preprocessor with config
    pub fn new(config: ProcessingConfig) -> Self {
        Self { config }
    }
    
    /// Process file and resolve includes
    pub fn process_file(&self, input_path: &Path, output_path: &Path) -> Result<()> {
        let mut context = ProcessingContext::new(self.config.clone());
        
        // Read input file
        let input_content = FileResolver::read_file_content(input_path)
            .with_context(|| format!("Failed to read input file: {}", input_path.display()))?;
        
        // Process file content
        let processed_content = self.process_content(&input_content, input_path, &mut context)?;
        
        // Write output
        std::fs::write(output_path, processed_content)
            .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;
        
        Ok(())
    }
    
    /// Process content and resolve includes recursively
    fn process_content(
        &self,
        content: &str,
        source_file: &Path,
        context: &mut ProcessingContext,
    ) -> Result<String> {
        // Enter file in context
        context.enter_file(source_file)?;
        
        // Parse include directives
        let includes = IncludeParser::parse_includes(content, &source_file.to_path_buf())?;
        
        let result = if includes.is_empty() {
            // No includes, return as-is
            content.to_string()
        } else {
            // Process file lines and replace includes
            let mut result = String::new();
            let lines: Vec<&str> = content.lines().collect();
            
            for (line_index, line) in lines.iter().enumerate() {
                let line_number = line_index + 1;
                
                // Check for include directive
                if let Some(include) = includes.iter().find(|inc| inc.line_number == line_number) {
                    // Replace with included content
                    let included_content = self.process_include(include, context)?;
                    result.push_str(&included_content);
                } else {
                    // Regular line
                    result.push_str(line);
                }
                
                // Add newline except last line
                if line_index < lines.len() - 1 {
                    result.push('\n');
                }
            }
            
            result
        };
        
        // Exit file from context
        context.exit_file();
        
        Ok(result)
    }
    
    /// Process single include directive
    fn process_include(
        &self,
        directive: &IncludeDirective,
        context: &mut ProcessingContext,
    ) -> Result<String> {
        // Resolve file path
        let resolved_path = FileResolver::resolve_include_path(directive, context.config())?;
        
        // Read included content
        let included_content = FileResolver::read_file_content(&resolved_path)?;
        
        // Process file included content recursively
        let processed_included = self.process_content(&included_content, &resolved_path, context)?;
        
        // Generate output with debug comments
        let mut result = String::new();
        
        if context.config().debug_mode {
            result.push_str(&FileResolver::generate_include_comment(&resolved_path, true));
            result.push('\n');
        }
        
        result.push_str(&processed_included);
        
        if context.config().debug_mode {
            // Add newline before end comment
            if !processed_included.ends_with('\n') {
                result.push('\n');
            }
            result.push_str(&FileResolver::generate_include_comment(&resolved_path, false));
        }
        
        Ok(result)
    }
}


/// Builder for preprocessor config
pub struct PreprocessorBuilder {
    config: ProcessingConfig,
}

impl PreprocessorBuilder {
    /// Create builder with defaults
    pub fn new() -> Self {
        Self {
            config: ProcessingConfig::default(),
        }
    }
    
    /// Enable debug mode
    pub fn debug_mode(mut self, enabled: bool) -> Self {
        self.config.debug_mode = enabled;
        self
    }
    
    /// Set max include depth
    pub fn max_include_depth(mut self, depth: usize) -> Self {
        self.config.max_include_depth = depth;
        self
    }
    
    /// Set base directory
    pub fn base_directory<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.config.base_directory = path.into();
        self
    }
    
    /// Build preprocessor
    pub fn build(self) -> ShellPreprocessor {
        ShellPreprocessor::new(self.config)
    }
}

impl Default for PreprocessorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[test]
    fn test_simple_include() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create include file
        let utils_path = temp_dir.path().join("utils.sh");
        fs::write(&utils_path, "function hello() {\n    echo \"Hello\"\n}").unwrap();
        
        // Create main file
        let main_path = temp_dir.path().join("main.sh");
        fs::write(&main_path, "#!/bin/bash\n#include utils.sh\nhello").unwrap();
        
        // Create output path
        let output_path = temp_dir.path().join("output.sh");
        
        // Process file
        let preprocessor = PreprocessorBuilder::new()
            .base_directory(temp_dir.path())
            .build();
        
        preprocessor.process_file(&main_path, &output_path).unwrap();
        
        // Check output
        let result = fs::read_to_string(&output_path).unwrap();
        assert!(result.contains("#!/bin/bash"));
        assert!(result.contains("function hello()"));
        assert!(result.contains("hello"));
    }
    
    #[test]
    fn test_debug_mode() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create include file
        let utils_path = temp_dir.path().join("utils.sh");
        fs::write(&utils_path, "echo \"utility\"").unwrap();
        
        // Create main file
        let main_path = temp_dir.path().join("main.sh");
        fs::write(&main_path, "#include utils.sh").unwrap();
        
        // Create output path
        let output_path = temp_dir.path().join("output.sh");
        
        // Process file with debug mode
        let preprocessor = PreprocessorBuilder::new()
            .debug_mode(true)
            .base_directory(temp_dir.path())
            .build();
        
        preprocessor.process_file(&main_path, &output_path).unwrap();
        
        // Check output contains debug comments
        let result = fs::read_to_string(&output_path).unwrap();
        assert!(result.contains("# --- Included from"));
        assert!(result.contains("# --- End of"));
    }
    
    #[test]
    fn test_recursive_include() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create nested included file
        let nested_path = temp_dir.path().join("nested.sh");
        fs::write(&nested_path, "echo \"nested\"").unwrap();
        
        // Create middle file that includes nested
        let middle_path = temp_dir.path().join("middle.sh");
        fs::write(&middle_path, "#include nested.sh\necho \"middle\"").unwrap();
        
        // Create main file that includes middle
        let main_path = temp_dir.path().join("main.sh");
        fs::write(&main_path, "#include middle.sh\necho \"main\"").unwrap();
        
        // Create output path
        let output_path = temp_dir.path().join("output.sh");
        
        // Process file
        let preprocessor = PreprocessorBuilder::new()
            .base_directory(temp_dir.path())
            .build();
        
        preprocessor.process_file(&main_path, &output_path).unwrap();
        
        // Check output
        let result = fs::read_to_string(&output_path).unwrap();
        assert!(result.contains("echo \"nested\""));
        assert!(result.contains("echo \"middle\""));
        assert!(result.contains("echo \"main\""));
    }
}