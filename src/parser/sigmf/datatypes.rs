// Put your SigMFDataType enum and related logic here
use anyhow::Result;
use num_complex::Complex;
use std::io::{Read, Cursor};

// SNW - small subset of the sigmf data types, because we only ever use these two anyway
#[derive(Debug, Clone)]
pub enum SigMFDataType {
    Cf32Le,
    Ci16Le,
}

impl SigMFDataType {
    pub fn from_string(s: &str) -> Result<Self> {
        match s {
            "cf32_le" => Ok(SigMFDataType::Cf32Le),
            "ci16_le" => Ok(SigMFDataType::Ci16Le),
            _ => Err(anyhow::anyhow!("Unsupported datatype: {}", s)),
        }
    }
    
    pub fn sample_size_bytes(&self) -> usize {
        match self {
            SigMFDataType::Cf32Le => 8, // 4 bytes for I + 4 bytes for Q
            SigMFDataType::Ci16Le => 4, // 2 bytes for I + 2 bytes for Q
        }
    }
    
    pub fn is_complex(&self) -> bool {
        return true; // Both cf32_le and ci16_le are complex types
    }
}
