use super::SigMFParser;
use anyhow::Result;
use polars::prelude::*;
use std::path::Path;
use walkdir::WalkDir;

pub struct SigMFDataset;

impl SigMFDataset {
    /// Parse all .sigmf-meta files in a directory and create a dataset DataFrame
    pub fn from_directory<P: AsRef<Path>>(dir_path: P) -> Result<DataFrame> {
        let mut all_rows = Vec::new();
        let mut processed_count = 0;
        let mut error_count = 0;
        
        println!("Scanning directory: {:?}", dir_path.as_ref());
        
        // Find all .sigmf-meta files
        for entry in WalkDir::new(dir_path).follow_links(true) {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("sigmf-meta") {
                processed_count += 1;
                if processed_count % 10 == 0 {
                    println!("Processed {} files...", processed_count);
                }
                
                match SigMFParser::from_meta_file(path) {
                    Ok(parser) => {
                        match parser.to_summary_row() {
                            Ok(row_df) => all_rows.push(row_df),
                            Err(e) => {
                                error_count += 1;
                                eprintln!("Failed to create summary for {:?}: {}", path, e);
                            }
                        }
                    }
                    Err(e) => {
                        error_count += 1;
                        eprintln!("Failed to parse {:?}: {}", path, e);
                    }
                }
            }
        }
        
        println!("Processed {} files, {} errors", processed_count, error_count);
        
        if all_rows.is_empty() {
            anyhow::bail!("No valid SigMF files found in directory");
        }
        
        // Combine all rows into one DataFrame using vstack
        let mut combined = all_rows.clone().into_iter().next().unwrap();
        for row_df in all_rows.into_iter().skip(1) {
            combined.vstack_mut(&row_df)?;
        }
        
        println!("Final dataset shape: {:?}", combined.shape());
        Ok(combined)
    }
    
    /// Parse specific files into a dataset
    pub fn from_files<P: AsRef<Path>>(file_paths: &[P]) -> Result<DataFrame> {
        if file_paths.is_empty() {
            anyhow::bail!("No files provided");
        }
        let mut all_rows = Vec::new();
        for path in file_paths {
            let parser = SigMFParser::from_meta_file(path)?;
            let row_df = parser.to_summary_row()?;
            all_rows.push(row_df);
        }
        let mut combined = all_rows.remove(0);
        for row_df in all_rows {
            combined.vstack_mut(&row_df)?;
        }
        Ok(combined)
    }
}
