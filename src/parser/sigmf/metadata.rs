use serde::{Deserialize,Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SigMFMetadata {
    pub global: GlobalInfo,
    pub captures: Vec<CaptureInfo>,
    pub annotations: Option<Vec<AnnotationInfo>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GlobalInfo {
    #[serde(rename = "core:datatype")]
    pub datatype: String,
    #[serde(rename = "core:sample_rate")]
    pub sample_rate: f64,
    #[serde(rename = "core:version")]
    pub version: String,
    #[serde(rename = "core:description")]
    pub description: Option<String>,
    #[serde(rename = "core:author")]
    pub author: Option<String>,
    #[serde(rename = "core:license")]
    pub license: Option<String>,
    #[serde(rename = "core:hw")]
    pub hardware: Option<String>,
    #[serde(rename = "core:geolocation")]
    pub geolocation: Option<GeoLocation>,
    
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeoLocation {
    #[serde(rename = "type")]
    pub geo_type: String,
    pub coordinates: Vec<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CaptureInfo {
    // SigMF Core Fields
    #[serde(rename = "core:sample_start")]
    pub sample_start: Option<u64>,
    #[serde(rename = "core:frequency")]
    pub frequency: Option<f64>,
    #[serde(rename = "core:datetime")]
    pub timestamp: Option<String>,

    // Distributed Spectrum Specific Fields
    #[serde(rename = "ds:agc")]
    pub agc: Option<bool>,
    #[serde(rename = "ds:gain")]
    pub gain: Option<f64>,
    #[serde(rename = "ds:sequence_num")]
    pub sequence_num: Option<u64>,  

    #[serde(flatten)]
    pub extra_fields: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnnotationInfo {
    // SigMF Core Fields
    #[serde(rename = "core:sample_start")]
    pub sample_start: u64,
    #[serde(rename = "core:sample_count")]
    pub sample_count: u64,
    #[serde(rename = "core:freq_lower_edge")]
    pub freq_lower_edge: Option<f64>,
    #[serde(rename = "core:freq_upper_edge")]
    pub freq_upper_edge: Option<f64>,

    // Distributed Spectrum Specific Fields
    //#[serde(rename = "ds:actually_using_wb_params")]
    //pub using_wb_params: Option<bool>,
    #[serde(rename = "ds:analogAmProb")]
    pub analog_am_prob: Option<f64>,
    #[serde(rename = "ds:analogFmProb")]
    pub analog_fm_prob: Option<f64>,
    #[serde(rename = "ds:askProb")]
    pub ask_prob: Option<f64>,
    #[serde(rename = "ds:fskProb")]
    pub fsk_prob: Option<f64>,
    #[serde(rename = "ds:pskProb")]
    pub psk_prob: Option<f64>,
    #[serde(rename = "ds:chirpProb")]
    pub chirp_prob: Option<f64>,
    #[serde(rename = "ds:constellationProb")]
    pub constellation_prob: Option<f64>,
    #[serde(rename = "ds:cssProb")]
    pub css_prob: Option<f64>,
    #[serde(rename = "ds:customClassifierProbs")]
    pub custom_classifier_probs: Option<Vec<CustomClassProbField>>,
    #[serde(rename = "ds:ml_no_sig")]
    pub ml_no_sig: Option<bool>,
    #[serde(rename = "ds:ook_prob")]
    pub ook_prob: Option<f64>,
    #[serde(rename = "ds:sdr_handle")]
    pub sdr_handle: Option<String>,
    #[serde(rename = "ds:sigBandwidth")]
    pub sig_bandwidth: Option<f64>,
    #[serde(rename = "ds:sigCenterFreq")]
    pub sig_center_freq: Option<f64>,
    #[serde(rename = "ds:sig_power_dbfs")]
    pub sig_power_dbfs : Option<f64>,
    #[serde(rename = "ds:sig_power_dbm")]
    pub sig_power_dbm : Option<f64>,
    #[serde(rename = "ds:snr")]
    pub sig_snr : Option<f64>,
    #[serde(rename = "ds:uuid")]
    pub uuid: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CustomClassProbField {
    #[serde(rename = "className")]
    pub class_name: String,
    #[serde(rename = "classProb")]
    pub class_prob: f32,
}

