use super::{SigMFDataType, SigMFMetadata};
use polars::prelude::*;
use anyhow::Result;
use std::path::Path;
use std::fs::File;

pub struct SigMFParser {
    pub metadata: SigMFMetadata,
    pub data_type: SigMFDataType,
    pub data_file_path: std::path::PathBuf,
}

impl SigMFParser{
    pub fn from_meta_file<P: AsRef<Path>>(meta_path: P) -> Result<Self> {
        let meta_path = meta_path.as_ref();

        let meta_content = std::fs::read_to_string(meta_path)?;
        let metadata: SigMFMetadata = serde_json::from_str(&meta_content)?;
        let data_type = SigMFDataType::from_string(&metadata.global.datatype)?;

        let data_file_path = meta_path.with_extension("sigmf-data");
        if !data_file_path.exists() {
            return Err(anyhow::anyhow!("Data file does not exist: {:?}", data_file_path));
        }
        Ok(SigMFParser {
            metadata,
            data_type,
            data_file_path,
        })
    }
    
    fn is_ml_annotation(&self, ann: &super::AnnotationInfo) -> bool {
        ann.sig_center_freq.is_some() || 
        ann.ask_prob.is_some() || 
        ann.psk_prob.is_some() ||
        ann.custom_classifier_probs.is_some()
    }

    // helper to get custom classifier probability for a specific annotation and class name 
    // TODO snw -- make this just iterate over all custom classifier and add columns dynamically
    fn get_custom_classifier_prob_for_annotation(
        &self, 
        ml_annotation: Option<&super::AnnotationInfo>, 
        class_name: &str
    ) -> Option<f64> {
        ml_annotation?
            .custom_classifier_probs.as_ref()?
            .iter()
            .find(|c| c.class_name == class_name)
            .map(|c| c.class_prob as f64)
    }

    pub fn to_summary_rows(&self) -> Result<DataFrame> {
        let global = &self.metadata.global;
        
        // Get data filename (not full path)
        let data_filename = self.data_file_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        
        let meta_filename = self.data_file_path
            .with_extension("sigmf-meta")
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        
        // Calculate basic file info
        let (num_samples, file_size_bytes) = if self.data_file_path.exists() {
            let file_size = std::fs::metadata(&self.data_file_path)?.len();
            let sample_size = self.data_type.sample_size_bytes() as u64;
            let num_samples = file_size / sample_size;
            (num_samples, file_size)
        } else {
            (0, 0)
        };

        // Get capture info (this remains the same for all rows)
        let capture_with_freq = self.metadata.captures.iter()
            .find(|c| c.frequency.is_some());
        let capture_with_datetime = self.metadata.captures.iter()
            .find(|c| c.timestamp.is_some());
        let capture_with_ds_info = self.metadata.captures.iter()
            .find(|c| c.gain.is_some() || c.agc.is_some());

        // Get ML annotations (annotations with ML data)
        let ml_annotations: Vec<_> = self.metadata.annotations.as_ref()
            .map(|anns| anns.iter().filter(|ann| self.is_ml_annotation(ann)).collect())
            .unwrap_or_default();

        // If no ML annotations, create a single row with default ML values
        if ml_annotations.is_empty() {
            return self.create_single_row_dataframe(
                &meta_filename,
                &data_filename,
                num_samples,
                file_size_bytes,
                global,
                capture_with_freq,
                capture_with_datetime,
                capture_with_ds_info,
                None,
            );
        }

        // Create a row for each ML annotation
        let mut all_rows = Vec::new();
        for ml_annotation in ml_annotations {
            let row_df = self.create_single_row_dataframe(
                &meta_filename,
                &data_filename,
                num_samples,
                file_size_bytes,
                global,
                capture_with_freq,
                capture_with_datetime,
                capture_with_ds_info,
                Some(ml_annotation),
            )?;
            all_rows.push(row_df);
        }

        // Concatenate all rows
        let mut result = all_rows[0].clone();
        for df in all_rows.into_iter().skip(1) {
            result = result.vstack(&df)?;
        }
        Ok(result)
    }

    fn create_single_row_dataframe(
        &self,
        meta_filename: &str,
        data_filename: &str,
        num_samples: u64,
        file_size_bytes: u64,
        global: &super::GlobalInfo,
        capture_with_freq: Option<&super::CaptureInfo>,
        capture_with_datetime: Option<&super::CaptureInfo>,
        capture_with_ds_info: Option<&super::CaptureInfo>,
        ml_annotation: Option<&super::AnnotationInfo>,
    ) -> Result<DataFrame> {
        let df = df! {
            // File identification
            "meta_filename" => vec![meta_filename.to_string()],
            "data_filename" => vec![data_filename.to_string()],
            
            // Basic file info
            "num_samples" => vec![num_samples],
            "file_size_bytes" => vec![file_size_bytes],
            "duration_s" => vec![num_samples as f64 / global.sample_rate],
            
            // Global metadata
            "sample_rate_hz" => vec![global.sample_rate],
            "datatype" => vec![global.datatype.clone()],
            "sigmf_version" => vec![global.version.clone()],
            "author" => vec![global.author.clone().unwrap_or_default()],
            "hardware" => vec![global.hardware.clone().unwrap_or_default()],
            
            // Geolocation
            "latitude" => vec![
                global.geolocation.as_ref()
                    .and_then(|g| g.coordinates.get(0))
                    .copied()
                    .unwrap_or(0.0)
            ],
            "longitude" => vec![
                global.geolocation.as_ref()
                    .and_then(|g| g.coordinates.get(1))
                    .copied()
                    .unwrap_or(0.0)
            ],
            "geo_type" => vec![
                global.geolocation.as_ref()
                    .map(|g| g.geo_type.clone())
                    .unwrap_or_default()
            ],
            
            // Capture information
            "center_freq_hz" => vec![
                capture_with_freq
                    .and_then(|c| c.frequency)
                    .unwrap_or(0.0)
            ],
            "capture_datetime" => vec![
                capture_with_datetime
                    .and_then(|c| c.timestamp.clone())
                    .unwrap_or_default()
            ],
            "gain" => vec![
                capture_with_ds_info
                    .and_then(|c| c.gain)
                    .unwrap_or(0.0)
            ],
            "agc" => vec![
                capture_with_ds_info
                    .and_then(|c| c.agc)
                    .unwrap_or(false)
            ],
            "sequence_num" => vec![
                capture_with_ds_info
                    .and_then(|c| c.sequence_num)
                    .unwrap_or(0)
            ],
            
            // Classical Signal Processing Derived Estimates
            "snr_db" => vec![ml_annotation.and_then(|a| a.sig_snr).unwrap_or(0.0)],
            "power_dbm" => vec![ml_annotation.and_then(|a| a.sig_power_dbm).unwrap_or(0.0)],
            "power_dbfs" => vec![ml_annotation.and_then(|a| a.sig_power_dbfs).unwrap_or(0.0)],
            "sig_bandwidth_hz" => vec![ml_annotation.and_then(|a| a.sig_bandwidth).unwrap_or(0.0)],
            "sig_center_freq_hz" => vec![ml_annotation.and_then(|a| a.sig_center_freq).unwrap_or(0.0)],
            
            // Modulation probabilities
            "ml_ask_prob" => vec![ml_annotation.and_then(|a| a.ask_prob).unwrap_or(0.0)],
            "ml_psk_prob" => vec![ml_annotation.and_then(|a| a.psk_prob).unwrap_or(0.0)],
            "ml_fsk_prob" => vec![ml_annotation.and_then(|a| a.fsk_prob).unwrap_or(0.0)],
            "ml_am_prob" => vec![ml_annotation.and_then(|a| a.analog_am_prob).unwrap_or(0.0)],
            "ml_fm_prob" => vec![ml_annotation.and_then(|a| a.analog_fm_prob).unwrap_or(0.0)],
            "ml_ook_prob" => vec![ml_annotation.and_then(|a| a.ook_prob).unwrap_or(0.0)],
            "ml_chirp_prob" => vec![ml_annotation.and_then(|a| a.chirp_prob).unwrap_or(0.0)],
            "ml_constellation_prob" => vec![ml_annotation.and_then(|a| a.constellation_prob).unwrap_or(0.0)],
            "ml_css_prob" => vec![ml_annotation.and_then(|a| a.css_prob).unwrap_or(0.0)],
            
            // Custom classifier results
            "ml_wifi_prob" => vec![self.get_custom_classifier_prob_for_annotation(ml_annotation, "wifi").unwrap_or(0.0)],
            "ml_cell_prob" => vec![self.get_custom_classifier_prob_for_annotation(ml_annotation, "cell").unwrap_or(0.0)],
            "ml_radar_prob" => vec![self.get_custom_classifier_prob_for_annotation(ml_annotation, "radar").unwrap_or(0.0)],
            
            // Boolean flags
            "ml_no_sig" => vec![ml_annotation.and_then(|a| a.ml_no_sig).unwrap_or(false)],
            
            // String identifiers
            "sig_uuid" => vec![ml_annotation.and_then(|a| a.uuid.clone()).unwrap_or_default()],
            "sdr_handle" => vec![ml_annotation.and_then(|a| a.sdr_handle.clone()).unwrap_or_default()],
            
            // Annotation frequency ranges
            "freq_lower_edge_hz" => vec![
                self.metadata.annotations.as_ref()
                    .and_then(|anns| anns.first())
                    .and_then(|ann| ann.freq_lower_edge)
                    .unwrap_or(0.0)
            ],
            "freq_upper_edge_hz" => vec![
                self.metadata.annotations.as_ref()
                    .and_then(|anns| anns.first())
                    .and_then(|ann| ann.freq_upper_edge)
                    .unwrap_or(0.0)
            ],
        }?;
        
        Ok(df)
    }

    pub fn to_summary_row(&self) -> Result<DataFrame> {
        self.to_summary_rows()
    }

    fn get_custom_classifier_prob(&self, class_name: &str) -> Option<f64> {
        self.metadata.annotations.as_ref()?
            .iter()
            .find_map(|ann| ann.custom_classifier_probs.as_ref()?
            .iter()
            .find(|c| c.class_name == class_name)
            .map(|c| c.class_prob as f64))
                
    }

    pub fn sample_rate(&self) -> f64 {
        self.metadata.global.sample_rate
    }

    pub fn get_annotations(&self) -> Option<&Vec<super::AnnotationInfo>> {
        self.metadata.annotations.as_ref()
    }

    pub fn get_captures(&self) -> &Vec<super::CaptureInfo> {
        &self.metadata.captures
    }
}

