pub mod sigmf;
// this is where we'd add other file types

pub use sigmf::{SigMFParser, SigMFDataset};

use anyhow::Result;
use polars::prelude::*;
use std::path::Path;

pub struct FileParser;

impl FileParser {
    pub fn parse_sigmf_summary<P: AsRef<Path>>(path: P) -> Result<DataFrame> {
        let parser = SigMFParser::from_meta_file(path)?;
        parser.to_summary_row()
    }

    pub fn parse_sigmf_directory<P: AsRef<Path>>(dir_path: P) -> Result<DataFrame> {
        SigMFDataset::from_directory(dir_path)
    }

    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<LazyFrame> {
        let path = path.as_ref();
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        match extension {
            "sigmf-meta" => {
                let summary_df = Self::parse_sigmf_summary(path)?;
                Ok(summary_df.lazy())
            }
            _ => anyhow::bail!("Unsupported file extension: {}", extension),
        }
    }

    pub fn parse_directory<P : AsRef<Path>>(dir_path: P) -> Result<LazyFrame> {
        let df = Self::parse_sigmf_directory(dir_path)?;
        Ok(df.lazy())
    }
}
