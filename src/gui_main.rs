use eframe::egui;
use polars::prelude::*;
use sig_viewer::parser::SigMFDataset;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("Sig Viewer"),
        ..Default::default()
    };

    eframe::run_native(
        "Sig Viewer",
        options,
        Box::new(|cc| {
            // Set light theme
            cc.egui_ctx.set_visuals(egui::Visuals::light());
            
            Ok(Box::new(SigViewerApp::new()))
        }),
    )
}

#[derive(Serialize, Deserialize, Default)]
struct AppConfig {
    last_directory: String,
    use_dark_theme: bool,
    hidden_columns: HashSet<String>,
    window_size: Option<[f32; 2]>,
}

impl AppConfig {
    fn config_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sig_viewer");
        
        std::fs::create_dir_all(&config_dir).ok();
        config_dir.join("config.json")
    }
    
    fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&contents) {
                    return config;
                }
            }
        }
        Self::default()
    }
    
    fn save(&self) {
        let path = Self::config_path();
        if let Ok(contents) = serde_json::to_string_pretty(self) {
            std::fs::write(path, contents).ok();
        }
    }
}

struct SigViewerApp {
    dataset: Option<DataFrame>,
    filtered_dataset: Option<DataFrame>,
    directory_path: String,
    status_message: String,
    column_filters: HashMap<String, String>,
    show_load_dialog: bool,
    error_message: Option<String>,
    file_dialog: egui_file::FileDialog,
    hidden_columns: HashSet<String>,
    show_column_selector: bool,
    config: AppConfig,
    use_dark_theme: bool,
    table_cache: Option<Vec<Vec<String>>>, // Cached formatted cell values
    cache_valid: bool,
    last_filter_hash: u64, // To detect when filters actually change
    visible_row_range: std::ops::Range<usize>, // Only render visible rows
    selected_row: Option<usize>, // Currently selected row
    show_visualization_dialog: bool,
    selected_row_data: Option<HashMap<String, String>>,
}

impl Default for SigViewerApp {
    fn default() -> Self {
        let config = AppConfig::load();
        
        Self {
            dataset: None,
            filtered_dataset: None,
            directory_path: config.last_directory.clone(),
            status_message: "No data loaded".to_string(),
            column_filters: HashMap::new(),
            show_load_dialog: true,
            error_message: None,
            file_dialog: egui_file::FileDialog::select_folder(
                if config.last_directory.is_empty() { 
                    None 
                } else { 
                    Some(PathBuf::from(&config.last_directory)) 
                }
            ),
            hidden_columns: config.hidden_columns.clone(),
            show_column_selector: false,
            use_dark_theme: config.use_dark_theme,
            config,
            table_cache: None,
            cache_valid: false,
            last_filter_hash: 0,
            visible_row_range: 0..0,
            selected_row: None,
            show_visualization_dialog: false,
            selected_row_data: None,
        }
    }
}

// main functionality impl block
impl SigViewerApp {
    fn new() -> Self {
        Self::default()
    }

    fn save_config(&mut self) {
        self.config.last_directory = self.directory_path.clone();
        self.config.use_dark_theme = self.use_dark_theme;
        self.config.hidden_columns = self.hidden_columns.clone();
        self.config.save();
    }
    
    fn invalidate_cache(&mut self) {
        self.cache_valid = false;
        self.table_cache = None;
    }

    fn build_table_cache(&mut self, dataset: &DataFrame, visible_columns: &[String]) {
        if self.cache_valid {
            return;
        }
        
        let num_rows = dataset.height().min(1000);
        let mut cache = Vec::with_capacity(num_rows);
        
        for row_idx in 0..num_rows {
            let mut row_cache = Vec::with_capacity(visible_columns.len());
            for column_name in visible_columns {
                if let Ok(column) = dataset.column(column_name) {
                    let cell_value = format_cell_value(column, row_idx);
                    row_cache.push(cell_value);
                } else {
                    row_cache.push("Error".to_string());
                }
            }
            cache.push(row_cache);
        }
        
        self.table_cache = Some(cache);
        self.cache_valid = true;
    }

    fn load_dataset(&mut self, path: &str) {
        self.status_message = "Loading...".to_string();
        self.error_message = None;
        
        match SigMFDataset::from_directory(path) {
            Ok(dataset) => {
                self.status_message = format!("Loaded {} files", dataset.height());
                
                // Initialize column filters
                self.column_filters.clear();
                for col_name in dataset.get_column_names() {
                    self.column_filters.insert(col_name.to_string(), String::new());
                }
                
                self.filtered_dataset = Some(dataset.clone());
                self.dataset = Some(dataset);
                self.invalidate_cache(); // Add this line
                self.show_load_dialog = false;
                
                // Save the successful directory path
                self.directory_path = path.to_string();
                self.save_config();
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to load dataset: {}", e));
                self.status_message = "Load failed".to_string();
            }
        }
    }

    fn apply_filters(&mut self) {
        let dataset = if let Some(ref dataset) = self.dataset {
            dataset.clone()
        } else {
            return;
        };
        
        // Create a hash of current filters to detect changes
        let current_hash = self.calculate_filter_hash();
        
        // Only recompute if filters actually changed
        if current_hash == self.last_filter_hash {
            return;
        }
        
        self.last_filter_hash = current_hash;
        
        let mut filtered = dataset.clone().lazy();
        
        // Apply filters
        for (column_name, filter_text) in &self.column_filters {
            if !filter_text.is_empty() {
                if let Ok(column) = dataset.column(column_name) {
                    match column.dtype() {
                        DataType::String => {
                            filtered = filtered.filter(
                                col(column_name).eq(lit(filter_text.clone()))
                            );
                        }
                        DataType::Float64 | DataType::Float32 => {
                            if let Ok(num) = filter_text.parse::<f64>() {
                                filtered = filtered.filter(col(column_name).gt_eq(lit(num)));
                            }
                        }
                        DataType::Int64 | DataType::Int32 | DataType::UInt64 | DataType::UInt32 => {
                            if let Ok(num) = filter_text.parse::<i64>() {
                                filtered = filtered.filter(col(column_name).gt_eq(lit(num)));
                            }
                        }
                        DataType::Boolean => {
                            if filter_text.to_lowercase() == "true" {
                                filtered = filtered.filter(col(column_name));
                            } else if filter_text.to_lowercase() == "false" {
                                filtered = filtered.filter(col(column_name).not());
                            }
                        }
                        _ => {
                            if let Ok(num) = filter_text.parse::<f64>() {
                                filtered = filtered.filter(col(column_name).eq(lit(num)));
                            }
                        }
                    }
                }
            }
        }
        
        match filtered.collect() {
            Ok(result) => {
                let result_height = result.height();
                self.filtered_dataset = Some(result);
                self.invalidate_cache();
                self.status_message = format!("Showing {} of {} files", 
                    result_height, 
                    dataset.height()
                );
            }
            Err(e) => {
                self.error_message = Some(format!("Filter error: {}", e));
                self.filtered_dataset = Some(dataset);
            }
        }
    }

    fn calculate_filter_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        
        // Create a sorted vector of key-value pairs for consistent hashing
        let mut filter_vec: Vec<(&String, &String)> = self.column_filters.iter().collect();
        filter_vec.sort_by_key(|&(key, _)| key);
        
        for (key, value) in filter_vec {
            key.hash(&mut hasher);
            value.hash(&mut hasher);
        }
        
        hasher.finish()
    }

    fn render_dataset_table(&mut self, ui: &mut egui::Ui) {
        let dataset = if let Some(ref dataset) = self.filtered_dataset {
            dataset.clone()
        } else {
            return;
        };
        
        let available_height = ui.available_height() - 150.0;
        
        // Selection info and buttons
        ui.horizontal(|ui| {
            if let Some(selected_idx) = self.selected_row {
                ui.label(format!("Selected row: {}", selected_idx + 1));
                
                if ui.button("Visualize").clicked() {
                    self.show_visualization_dialog = true;
                }
                if ui.button("Open in Inspectrum").clicked() {
                    self.open_in_inspectrum();
                }
                if ui.button("Clear Selection").clicked() {
                    self.clear_selection();
                }
            } else {
                ui.label("No row selected");
            }
        });
        
        ui.separator();
        
        // Store selection changes to apply after table rendering
        let mut selection_change: Option<Option<usize>> = None;
        
        egui::ScrollArea::both()
            .max_height(available_height)
            .show(ui, |ui| {
                // Filter inputs
                ui.horizontal(|ui| {
                    ui.label("Filters:");
                    if ui.button("Columns...").clicked() {
                        self.show_column_selector = true;
                    }
                    if ui.button("Apply Filters").clicked() {
                        self.apply_filters();
                        self.invalidate_cache();
                        self.clear_selection();
                    }
                });
                
                let visible_columns = self.get_visible_columns(&dataset);
                
                // Filter boxes
                ui.horizontal_wrapped(|ui| {
                    for column_name_str in visible_columns.iter().take(8) {
                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                ui.strong(column_name_str);
                                let filter_text = self.column_filters.get_mut(column_name_str).unwrap();
                                let response = ui.text_edit_singleline(filter_text);
                                
                                if response.changed() {
                                    self.apply_filters();
                                    self.clear_selection();
                                }
                            });
                        });
                    }
                    
                    if visible_columns.len() > 8 {
                        ui.label(format!("... and {} more columns", visible_columns.len() - 8));
                    }
                });
                
                ui.separator();
                
                // Build cache if needed
                if !self.cache_valid || self.table_cache.is_none() {
                    self.build_table_cache(&dataset, &visible_columns);
                }
                
                // Table with selection
                use egui_extras::{Column, TableBuilder};
                
                let num_columns = visible_columns.len();
                
                if num_columns > 0 {
                    TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Column::exact(30.0)) // Selection column
                        .columns(Column::auto().at_least(100.0), num_columns)
                        .header(25.0, |mut header| {
                            header.col(|ui| {
                                ui.strong("Select");
                            });
                            for column_name in &visible_columns {
                                header.col(|ui| {
                                    ui.strong(column_name);
                                });
                            }
                        })
                        .body(|body| {
                            let cache = self.table_cache.as_ref();
                            let current_selection = self.selected_row;
                            
                            if let Some(cache) = cache {
                                body.rows(20.0, cache.len(), |mut row| {
                                    let row_index = row.index();
                                    let is_selected = current_selection == Some(row_index);
                                    
                                    // Selection column - try a different approach
                                    row.col(|ui| {
                                        // Add some debug visual feedback
                                        if ui.selectable_label(is_selected, if is_selected { "●" } else { "○" }).clicked() {
                                            if is_selected {
                                                selection_change = Some(None); // Clear selection
                                            } else {
                                                selection_change = Some(Some(row_index)); // Select this row
                                            }
                                        }
                                    });
                                    
                                    // Data columns
                                    if let Some(row_data) = cache.get(row_index) {
                                        for cell_value in row_data {
                                            row.col(|ui| {
                                                ui.label(cell_value);
                                            });
                                        }
                                    }
                                });
                            }
                        });
                } else {
                    ui.label("No visible columns. Use 'Columns...' to show some columns.");
                }
            });
        
        // Apply selection change after table rendering
        if let Some(new_selection) = selection_change {
            match new_selection {
                Some(row_idx) => self.select_row(row_idx),
                None => self.clear_selection(),
            }
        }
    }

    fn render_load_dialog(&mut self, ctx: &egui::Context) {
        if self.show_load_dialog {
            egui::Window::new("Load Dataset")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.heading("Load SigMF Dataset");
                    
                    ui.horizontal(|ui| {
                        ui.label("Directory:");
                        ui.text_edit_singleline(&mut self.directory_path);
                    });
                    
                    ui.horizontal(|ui| {
                        if ui.button("Load").clicked() && !self.directory_path.is_empty() {
                            self.load_dataset(&self.directory_path.clone());
                        }
                        
                        if ui.button("Browse...").clicked() {
                            self.file_dialog.open();
                        }
                    });
                    
                    if let Some(ref error) = self.error_message {
                        ui.colored_label(egui::Color32::RED, error);
                    }
                });
        }

        // Handle file dialog
        if self.file_dialog.show(ctx).selected() {
            if let Some(path) = self.file_dialog.path() {
                self.directory_path = path.to_string_lossy().to_string();
                // Update the file dialog's default path for next time
                self.file_dialog = egui_file::FileDialog::select_folder(Some(path.to_path_buf()));
            }
        }
    }
    fn render_column_selector(&mut self, ctx: &egui::Context) {
        if self.show_column_selector {
            egui::Window::new("Column Visibility")
                .collapsible(false)
                .resizable(true)
                .default_size([300.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Show/Hide Columns");
                    
                    // Clone the column names first to avoid borrowing issues
                    let column_names: Vec<String> = if let Some(ref dataset) = self.dataset {
                        dataset.get_column_names()
                            .iter()
                            .map(|s| s.to_string())
                            .collect()
                    } else {
                        Vec::new()
                    };
                    
                    if !column_names.is_empty() {
                        let mut changes_made = false;
                        
                        egui::ScrollArea::vertical()
                            .max_height(300.0)
                            .show(ui, |ui| {
                                for column_name in &column_names {
                                    let mut is_visible = !self.hidden_columns.contains(column_name);
                                    
                                    if ui.checkbox(&mut is_visible, column_name).changed() {
                                        if is_visible {
                                            self.hidden_columns.remove(column_name);
                                        } else {
                                            self.hidden_columns.insert(column_name.clone());
                                        }
                                        changes_made = true;
                                    }
                                }
                            });
                        if changes_made {
                            self.invalidate_cache(); // Add this line
                            self.save_config();
                        }
                        
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("Show All").clicked() {
                                self.hidden_columns.clear();
                                self.invalidate_cache();
                                self.save_config();
                            }
                            if ui.button("Hide All").clicked() {
                                for col in &column_names {
                                    self.hidden_columns.insert(col.clone());
                                }
                                self.save_config();
                            }
                        });
                    }
                    
                    if ui.button("Close").clicked() {
                        self.invalidate_cache();
                        self.show_column_selector = false;
                    }
                });
        }
    }
    fn get_visible_columns(&self, dataset: &DataFrame) -> Vec<String> {
        dataset.get_column_names()
            .iter()
            .map(|s| s.to_string())
            .filter(|col_name| !self.hidden_columns.contains(col_name))
            .collect()
    }
}

impl eframe::App for SigViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply theme if it changed
        if self.use_dark_theme != self.config.use_dark_theme {
            if self.use_dark_theme {
                ctx.set_visuals(egui::Visuals::dark());
            } else {
                ctx.set_visuals(egui::Visuals::light());
            }
            self.save_config();
        }

        // Top menu bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Load Directory...").clicked() {
                        self.show_load_dialog = true;
                        ui.close();
                    }
                    if ui.button("Export CSV...").clicked() {
                        // TODO: Implement CSV export
                        ui.close();
                    }
                });
                
                ui.menu_button("View", |ui| {
                    if ui.button("Clear Filters").clicked() {
                        for filter in self.column_filters.values_mut() {
                            filter.clear();
                        }
                        if self.dataset.is_some() {
                            self.filtered_dataset = self.dataset.clone();
                            self.status_message = format!("Showing all {} files", 
                                self.dataset.as_ref().unwrap().height());
                        }
                        ui.close();
                    }
                    if ui.button("Column Visibility...").clicked() {
                        self.show_column_selector = true;
                        ui.close();
                    }
                    
                    ui.separator();
                    if ui.checkbox(&mut self.use_dark_theme, "Dark Theme").changed() {
                        if self.use_dark_theme {
                            ctx.set_visuals(egui::Visuals::dark());
                        } else {
                            ctx.set_visuals(egui::Visuals::light());
                        }
                        self.save_config();
                    }
                });
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(&self.status_message);
                });
            });
        });

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.dataset.is_some() {
                self.render_dataset_table(ui);
            } else {
                ui.vertical_centered(|ui| {
                    ui.heading("Sig Viewer");
                    ui.label("Load a dataset to get started");
                    if ui.button("Load Dataset").clicked() {
                        self.show_load_dialog = true;
                    }
                });
            }
        });

        // Dialogs
        self.render_load_dialog(ctx);
        self.render_column_selector(ctx);
        self.render_visualization_dialog(ctx);
        
        // Error popup
        let show_error = self.error_message.is_some();
        if show_error {
            let error_msg = self.error_message.clone().unwrap();
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.colored_label(egui::Color32::RED, &error_msg);
                    if ui.button("OK").clicked() {
                        self.error_message = None;
                    }
                });
        }
    }
}

fn format_cell_value(column: &polars::series::Series, row_idx: usize) -> String {
    match column.dtype() {
        DataType::String => {
            column.str().unwrap().get(row_idx).unwrap_or("").to_string()
        }
        DataType::Float64 => {
            if let Some(val) = column.f64().unwrap().get(row_idx) {
                if val.abs() > 1000.0 || (val.abs() < 0.01 && val != 0.0) {
                    format!("{:.2e}", val)
                } else {
                    format!("{:.3}", val)
                }
            } else {
                "null".to_string()
            }
        }
        DataType::Float32 => {
            if let Some(val) = column.f32().unwrap().get(row_idx) {
                if val.abs() > 1000.0 || (val.abs() < 0.01 && val != 0.0) {
                    format!("{:.2e}", val)
                } else {
                    format!("{:.3}", val)
                }
            } else {
                "null".to_string()
            }
        }
        DataType::Int64 => {
            column.i64().unwrap().get(row_idx).map_or("null".to_string(), |v| v.to_string())
        }
        DataType::UInt64 => {
            column.u64().unwrap().get(row_idx).map_or("null".to_string(), |v| v.to_string())
        }
        DataType::Boolean => {
            column.bool().unwrap().get(row_idx).map_or("null".to_string(), |v| v.to_string())
        }
        _ => {
            format!("{:?}", column.get(row_idx).unwrap())
        }
    }
}


// handle selectable rows
impl SigViewerApp {
    fn select_row(&mut self, row_index: usize) {
    println!("Selecting row: {}", row_index); // Debug output
    self.selected_row = Some(row_index);
    
    // Use filtered_dataset instead of dataset
    if let Some(ref dataset) = self.filtered_dataset {
        let mut row_data = HashMap::new();
        
        // Make sure row_index is valid
        if row_index < dataset.height() {
            for column_name in dataset.get_column_names() {
                if let Ok(column) = dataset.column(column_name) {
                    let cell_value = format_cell_value(column, row_index);
                    row_data.insert(column_name.to_string(), cell_value);
                }
            }
            self.selected_row_data = Some(row_data);
            println!("Row data cached for row {}", row_index); // Debug output
        } else {
            println!("Row index {} out of bounds (dataset height: {})", row_index, dataset.height());
            self.selected_row_data = None;
        }
    } else {
        self.selected_row_data = None;
        println!("No filtered dataset available");
    }
    }

    fn clear_selection(&mut self) {
        self.selected_row = None;
        self.selected_row_data = None;
    }

    fn render_visualization_dialog(&mut self, ctx: &egui::Context) {
        if self.show_visualization_dialog {
            egui::Window::new("Visualize Signal Data")
                .collapsible(false)
                .resizable(true)
                .default_size([600.0, 400.0])
                .show(ctx, |ui| {
                    ui.heading("Signal Visualization");
                    
                    if let Some(ref row_data) = self.selected_row_data {
                        ui.separator();
                        
                        // Show key signal parameters
                        ui.label("Selected Signal Parameters:");
                        
                        egui::ScrollArea::vertical()
                            .max_height(200.0)
                            .show(ui, |ui| {
                                egui::Grid::new("signal_params")
                                    .num_columns(2)
                                    .spacing([20.0, 4.0])
                                    .show(ui, |ui| {
                                        // Show important parameters first
                                        let important_params = [
                                            ("meta_filename", "Filename"),
                                            ("sig_center_freq_hz", "Center Frequency (Hz)"),
                                            ("sample_rate_hz", "Sample Rate (Hz)"),
                                            ("sig_bandwidth_hz", "Bandwidth (Hz)"),
                                            ("snr_db", "SNR (dB)"),
                                            ("power_dbm", "Power (dBm)"),
                                            ("duration_s", "Duration (s)"),
                                            ("ml_wifi_prob", "WiFi Probability"),
                                            ("ml_cell_prob", "Cellular Probability"),
                                            ("ml_radar_prob", "Radar Probability"),
                                        ];
                                        
                                        for (key, display_name) in &important_params {
                                            if let Some(value) = row_data.get(*key) {
                                                ui.label(format!("{}:", display_name));
                                                ui.label(value);
                                                ui.end_row();
                                            }
                                        }
                                    });
                            });
                        
                        ui.separator();
                        
                        // Placeholder for actual visualization buttons
                        ui.horizontal(|ui| {
                            
                            if ui.button("PSD").clicked() {
                                // TODO: Implement frequency domain visualization
                                println!("Frequency domain plot requested for: {:?}", row_data.get("meta_filename"));
                            }
                            
                            if ui.button("Spectrogram").clicked() {
                                // TODO: Implement spectrogram visualization
                                println!("Spectrogram requested for: {:?}", row_data.get("meta_filename"));
                            }
                        });
                        
                        ui.separator();
                        ui.label("Note: Visualization functionality will load and process the actual signal data file.");
                        
                    } else {
                        ui.colored_label(egui::Color32::RED, "No row data available");
                    }
                    
                    ui.separator();
                    if ui.button("Close").clicked() {
                        self.show_visualization_dialog = false;
                    }
                });
        }
    }
}

// handle visualizations
impl SigViewerApp {
    fn open_in_inspectrum(&self) {
        if let Some(ref row_data) = self.selected_row_data {
            if let Some(meta_filename) = row_data.get("meta_filename") {
                // Get the full path to the meta file
                let meta_path = std::path::Path::new(&self.directory_path).join(meta_filename);
                
                // Launch inspectrum with the meta file path
                match std::process::Command::new("inspectrum")
                    .arg(meta_path.to_string_lossy().to_string())
                    .spawn()
                {
                    Ok(_) => {
                        println!("Launched inspectrum with: {}", meta_path.display());
                    }
                    Err(e) => {
                        println!("Failed to launch inspectrum");
                    }
                }
            } else {
                println!("No meta filename found in selected row data");
            }
        } else {
            println!("No row selected or row data not available");
        }
    }
}