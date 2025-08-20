//! Include directive parser

use std::path::PathBuf;
use crate::error::{PreprocessorError, Result};

/// Quote types for include directives
#[derive(Debug, Clone, PartialEq)]
pub enum IncludeQuoteType {
    /// <filepath>
    AngleBrackets,
    /// "filepath"
    DoubleQuotes,
    /// 'filepath'
    SingleQuotes,
    /// filepath
    None,
}

/// Parsed include directive
#[derive(Debug, Clone, PartialEq)]
pub struct IncludeDirective {
    /// Line number (1-indexed)
    pub line_number: usize,
    /// File path to include
    pub file_path: String,
    /// Source file path
    pub source_file: PathBuf,
    /// Quote type used
    pub quote_type: IncludeQuoteType,
}

impl IncludeDirective {
    /// Create a new include directive
    pub fn new(
        line_number: usize,
        file_path: String,
        source_file: PathBuf,
        quote_type: IncludeQuoteType,
    ) -> Self {
        Self {
            line_number,
            file_path,
            source_file,
            quote_type,
        }
    }
}

/// Include directive parser
pub struct IncludeParser;

impl IncludeParser {
    /// Parse all include directives from content
    pub fn parse_includes(content: &str, source_file: &PathBuf) -> Result<Vec<IncludeDirective>> {
        let mut directives = Vec::new();
        
        for (line_number, line) in content.lines().enumerate() {
            let line_number = line_number + 1;
            let trimmed = line.trim();
            
            if let Some(directive) = Self::parse_single_include(trimmed, line_number, source_file)? {
                directives.push(directive);
            }
        }
        
        Ok(directives)
    }
    
    /// Parse single line for include directive
    fn parse_single_include(
        line: &str,
        line_number: usize,
        source_file: &PathBuf,
    ) -> Result<Option<IncludeDirective>> {
        // Skip non-include lines
        if !line.starts_with("#include") {
            return Ok(None);
        }
        
        let after_include = line.strip_prefix("#include")
            .ok_or_else(|| PreprocessorError::InvalidIncludeDirective {
                line_number,
                directive: line.to_string(),
            })?
            .trim();
        
        if after_include.is_empty() {
            return Err(PreprocessorError::InvalidIncludeDirective {
                line_number,
                directive: line.to_string(),
            }.into());
        }
        
        // Parse quote types
        let (file_path, quote_type) = Self::extract_filepath_and_quote_type(after_include)
            .ok_or_else(|| PreprocessorError::InvalidIncludeDirective {
                line_number,
                directive: line.to_string(),
            })?;
        
        Ok(Some(IncludeDirective::new(
            line_number,
            file_path,
            source_file.clone(),
            quote_type,
        )))
    }
    
    /// Extract filepath and quote type
    fn extract_filepath_and_quote_type(input: &str) -> Option<(String, IncludeQuoteType)> {
        let input = input.trim();
        
        // Try angle brackets
        if input.starts_with('<') && input.ends_with('>') && input.len() > 2 {
            let path = &input[1..input.len()-1];
            if !path.is_empty() {
                return Some((path.to_string(), IncludeQuoteType::AngleBrackets));
            }
        }
        
        // Try double quotes
        if input.starts_with('"') && input.ends_with('"') && input.len() > 2 {
            let path = &input[1..input.len()-1];
            if !path.is_empty() {
                return Some((path.to_string(), IncludeQuoteType::DoubleQuotes));
            }
        }
        
        // Try single quotes
        if input.starts_with('\'') && input.ends_with('\'') && input.len() > 2 {
            let path = &input[1..input.len()-1];
            if !path.is_empty() {
                return Some((path.to_string(), IncludeQuoteType::SingleQuotes));
            }
        }
        
        // No quotes
        if !input.contains(char::is_whitespace) && !input.is_empty() 
            && !input.contains('<') && !input.contains('>') 
            && !input.contains('"') && !input.contains('\'') {
            return Some((input.to_string(), IncludeQuoteType::None));
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_angle_brackets() {
        let result = IncludeParser::extract_filepath_and_quote_type("<utils/helper.sh>");
        assert_eq!(result, Some(("utils/helper.sh".to_string(), IncludeQuoteType::AngleBrackets)));
    }
    
    #[test]
    fn test_parse_double_quotes() {
        let result = IncludeParser::extract_filepath_and_quote_type("\"config/settings.sh\"");
        assert_eq!(result, Some(("config/settings.sh".to_string(), IncludeQuoteType::DoubleQuotes)));
    }
    
    #[test]
    fn test_parse_single_quotes() {
        let result = IncludeParser::extract_filepath_and_quote_type("'helpers/logger.sh'");
        assert_eq!(result, Some(("helpers/logger.sh".to_string(), IncludeQuoteType::SingleQuotes)));
    }
    
    #[test]
    fn test_parse_no_quotes() {
        let result = IncludeParser::extract_filepath_and_quote_type("common.sh");
        assert_eq!(result, Some(("common.sh".to_string(), IncludeQuoteType::None)));
    }
    
    #[test]
    fn test_parse_invalid() {
        assert_eq!(IncludeParser::extract_filepath_and_quote_type(""), None);
        assert_eq!(IncludeParser::extract_filepath_and_quote_type("file with spaces"), None);
        assert_eq!(IncludeParser::extract_filepath_and_quote_type("<>"), None);
        assert_eq!(IncludeParser::extract_filepath_and_quote_type("\"\""), None);
        assert_eq!(IncludeParser::extract_filepath_and_quote_type("''"), None);
    }
}