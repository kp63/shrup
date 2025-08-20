//! Shell script preprocessor

use std::path::PathBuf;
use anyhow::Result;
use clap::Parser;

use shrup::PreprocessorBuilder;

/// Shell script preprocessor
#[derive(Parser)]
#[command(name = "shrup")]
#[command(version = "0.1.0")]
#[command(about = "A shell script preprocessor")]
#[command(long_about = None)]
struct Args {
    /// Input file to process
    #[arg(value_name = "INPUT")]
    input: PathBuf,
    
    /// Output file path
    #[arg(value_name = "OUTPUT")]
    output: PathBuf,
    
    /// Add debug comments to output
    #[arg(short, long)]
    debug: bool,
    
    /// Max include depth (default: 100)
    #[arg(long, default_value = "100")]
    max_depth: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    if !args.input.exists() {
        anyhow::bail!("Input file does not exist: {}", args.input.display());
    }
    
    if !args.input.is_file() {
        anyhow::bail!("Input path is not a file: {}", args.input.display());
    }
    
    // Get base directory
    let base_directory = args.input
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf();
    
    // Create preprocessor
    let preprocessor = PreprocessorBuilder::new()
        .debug_mode(args.debug)
        .max_include_depth(args.max_depth)
        .base_directory(base_directory)
        .build();
    
    // Process file
    match preprocessor.process_file(&args.input, &args.output) {
        Ok(()) => {
            if args.debug {
                eprintln!("✓ Successfully processed {} -> {}", 
                         args.input.display(), 
                         args.output.display());
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            
            // Print error chain
            let mut source = e.source();
            while let Some(err) = source {
                eprintln!("  Caused by: {}", err);
                source = err.source();
            }
            
            std::process::exit(1);
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[test]
    fn test_args_parsing() {
        // Test basic argument parsing
        let args = Args::try_parse_from(&["shrup", "input.sh", "output.sh"]).unwrap();
        assert_eq!(args.input, PathBuf::from("input.sh"));
        assert_eq!(args.output, PathBuf::from("output.sh"));
        assert_eq!(args.debug, false);
        assert_eq!(args.max_depth, 100);
    }
    
    #[test]
    fn test_args_with_debug() {
        let args = Args::try_parse_from(&["shrup", "--debug", "input.sh", "output.sh"]).unwrap();
        assert_eq!(args.debug, true);
    }
    
    #[test]
    fn test_args_with_max_depth() {
        let args = Args::try_parse_from(&["shrup", "--max-depth", "50", "input.sh", "output.sh"]).unwrap();
        assert_eq!(args.max_depth, 50);
    }
    
    #[test]
    fn test_integration_basic() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create input file
        let input_path = temp_dir.path().join("input.sh");
        fs::write(&input_path, "#!/bin/bash\necho \"Hello World\"").unwrap();
        
        let output_path = temp_dir.path().join("output.sh");
        
        // Run preprocessor
        let preprocessor = PreprocessorBuilder::new()
            .base_directory(temp_dir.path())
            .build();
        
        preprocessor.process_file(&input_path, &output_path).unwrap();
        
        // Check output
        let output_content = fs::read_to_string(&output_path).unwrap();
        assert!(output_content.contains("#!/bin/bash"));
        assert!(output_content.contains("echo \"Hello World\""));
    }
}
