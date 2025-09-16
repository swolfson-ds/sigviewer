mod parser;
//mod data_ops;
//mod viz;
//mod file_picker;

use clap::{Parser, Subcommand};
use anyhow::Result;
use parser::{FileParser, SigMFDataset};
use polars::prelude::*;
#[derive(Parser)]
#[command(name = "sig_viewer_cli")]
#[command(about = "A CLI tool for exploring RF data files")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Parse { 
        #[arg(help = "File or directory to parse")]
        path: String 
    },
    Dataset {
        #[arg(help = "Directory containing SigMF files")]
        dir: String,
        #[arg(long, help = "Output CSV file")]
        output: Option<String>,
    },
    Stats {
        #[arg(help = "Dataset CSV file")]
        dataset: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Parse { path } => {
            println!("Parsing: {}", path);
            let df = FileParser::parse_file(&path)?;
            let collected = df.collect()?;
            println!("Result: {} rows, {} columns", 
                collected.height(), 
                collected.width());
            println!("Columns: {:?}", collected.get_column_names());
            // for each column name print the first value
            for name in collected.get_column_names() {
                if let Ok(series) = collected.column(name) {
                    if series.len() > 0 {
                        println!("{}: {:?}", name, series.get(0));
                    }
                }
            }
        }
        
        Commands::Dataset { dir, output } => {
            println!("Building dataset from directory: {}", dir);
            let dataset = SigMFDataset::from_directory(&dir)?;
            
            println!("Dataset shape: {:?}", dataset.shape());
            
            if let Some(output_path) = output {
                use polars::prelude::*;
                let mut file = std::fs::File::create(&output_path)?;
                CsvWriter::new(&mut file).finish(&mut dataset.clone())?;
                println!("Saved dataset to: {}", output_path);
            } else {
                println!("First 5 rows:");
                println!("{}", dataset.head(Some(5)));
            }
        }
        
        Commands::Stats { dataset } => {
            println!("Loading dataset: {}", dataset);
            let lf = LazyCsvReader::new(dataset).finish()?;
            let stats = lf.select([
                col("ml_wifi_prob").mean().alias("avg_wifi_prob"),
                col("ml_snr_db").mean().alias("avg_snr"),
                col("center_freq_hz").n_unique().alias("unique_freqs"),
            ]).collect()?;
            
            println!("Dataset statistics:");
            println!("{}", stats);
        }
    }
    
    Ok(())
}
