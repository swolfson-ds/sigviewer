# Data Explorer

A Rust-based tool for exploring and analyzing RF signal data, with a focus on SigMF (Signal Metadata Format) files.

## Overview

Sigviewer is designed to make RF dataset exploration fast and intuitive. It is intended to replace the current process of grepping through sigmf metas in a directory for samples containing particular parameters, then opening the data file in inspectrum. It parses signal metadata and IQ data into structured DataFrames for easy analysis, with plans to add rich visualization and interactive browsing capabilities. 

The dataframe csv can also be imported into polars or pandas running in a jupyter lab environment for a more dynamic exploration experience.

## Current Features

### SigMF Parsing
- **Single file parsing**: Convert individual `.sigmf-meta` files into DataFrame rows containing all metadata
- **Batch directory parsing**: Process entire directories of SigMF files into a unified dataset
- **Rich metadata extraction**: Captures all standard SigMF fields plus vendor-specific extensions
- **ML classification data**: Full support for Distributed Spectrum ML annotations (modulation probabilities, SNR, power measurements, etc.)

### Data Structure
Each SigMF file becomes one row in the resulting DataFrame with columns including:
- File identification (`data_filename`, `meta_filename`) 
- Signal parameters (`sample_rate_hz`, `center_freq_hz`)
- Hardware info (`hardware`, `gain`, `agc`, `sdr_handle`)
- Geolocation (`latitude`, `longitude`)
- ML classifications (`ml_wifi_prob`, `ml_cell_prob`, `ml_radar_prob`, etc.)
- Modulation probabilities (`ml_ask_prob`, `ml_psk_prob`, `ml_fsk_prob`)

## Installation

```bash
git clone https://github.com/yourusername/sigviewer
cd sigviewer
cargo build --release
```

## Usage

### Parse a single SigMF file
```bash
cargo run -- parse capture.sigmf-meta
```

### Build a dataset from a directory
```bash
# Create dataset and display summary
cargo run -- dataset /path/to/sigmf/directory

# Save dataset to CSV
cargo run -- dataset /path/to/sigmf/directory --output dataset.csv
```

### Show dataset statistics
```bash
cargo run -- stats dataset.csv
```

### Programmatic usage
```rust
use data_explorer::parser::{SigMFParser, SigMFDataset};

// Parse single file
let parser = SigMFParser::from_meta_file("capture.sigmf-meta")?;
let summary = parser.to_summary_row()?;

// Parse directory into dataset  
let dataset = SigMFDataset::from_directory("/path/to/sigmf/files")?;
println!("Found {} captures", dataset.height());

// Query the data
let wifi_captures = dataset.filter(&col("ml_wifi_prob").gt(lit(0.8)))?;
```

## Project Structure

```
src/
├── main.rs              # CLI interface
├── parser/              # File parsing modules
│   ├── mod.rs          # Main parser interface
│   └── sigmf/          # SigMF-specific parsing
│       ├── metadata.rs  # SigMF metadata structures
│       ├── datatypes.rs # Data type handling  
│       ├── parser.rs    # Core SigMF parsing logic
│       └── dataset.rs   # Multi-file dataset creation
├── data_ops/           # Data operations (planned)
├── viz/                # Visualization (planned)  
└── file_picker.rs      # File utilities (planned)
```

## Dependencies

- **[Polars](https://pola.rs/)**: Fast DataFrames for data manipulation and analysis
- **[Serde](https://serde.rs/)**: JSON serialization/deserialization for SigMF metadata
- **[Anyhow](https://github.com/dtolnay/anyhow)**: Flexible error handling
- **[Walkdir](https://github.com/BurntSushi/walkdir)**: Directory traversal
- **[Clap](https://clap.rs/)**: Command-line interface

## Roadmap

### Signal Processing & Visualization 
- **Spectrogram generation**: Create time-frequency plots from IQ data
- **Power Spectral Density (PSD)**: Generate frequency domain representations
- **Waterfall plots**: Time-stacked spectrograms for long captures
- **IQ constellation diagrams**: Visualize modulation characteristics

### Integrate Additional Processing
- **Interface for Additional Processing Flows**: Create a way to hook in custom processing for prototyping and analysis
- **Training Data Selection for Model training**: Add a way for a user to label data snapshots

### Enhanced Plotting  
- **Parameter overlay**: Display calculated center frequency and bandwidth bounds on spectrograms
- **Interactive plots**: Zoom, pan, and measure directly on visualizations
- **Multi-signal comparison**: Overlay multiple captures for comparative analysis
- **Export capabilities**: Save plots as PNG or JPEG

### Dynamic Data Exploration 
- **Interactive DataFrame browser**: GUI for filtering and querying datasets
- **Real-time filtering**: Dynamic query interface with immediate visual feedback

### Advanced Analytics 
- **Statistical summaries**: Automated dataset characterization
- **Anomaly detection**: Identify unusual signals in large datasets
- **Classification validation**: Compare ML predictions with manual annotations
- **Batch processing**: Parallel processing of large dataset collections



---