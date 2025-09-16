use eframe::egui;
use polars::prelude::*;
use sig_viewer::parser::SigMFDataset;
use anyhow::Result;
use std::collections::{HashMap, HashSet};

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
        Box::new(|_cc| Ok(Box::new(SigViewerApp::new()))),
    )
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
    use_dark_theme: bool,
}

impl Default for SigViewerApp {
    fn default() -> Self {
        Self {
            dataset: None,
            filtered_dataset: None,
            directory_path: String::new(),
            status_message: "No data loaded".to_string(),
            column_filters: HashMap::new(),
            show_load_dialog: true,
            error_message: None,
            file_dialog: egui_file::FileDialog::select_folder(None),
            hidden_columns: HashSet::new(),
            show_column_selector: false,
            use_dark_theme: false,
        }
    }
}

impl SigViewerApp {
    fn new() -> Self {
        Self::default()
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
                self.show_load_dialog = false;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to load dataset: {}", e));
                self.status_message = "Load failed".to_string();
            }
        }
    }

    fn apply_filters(&mut self) {
        if let Some(ref dataset) = self.dataset {
            let mut filtered = dataset.clone().lazy();
            
            // Apply text filters for each column
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
                    self.filtered_dataset = Some(result.clone());
                    self.status_message = format!("Showing {} of {} files", 
                        result.height(), 
                        dataset.height()
                    );
                }
                Err(e) => {
                    self.error_message = Some(format!("Filter error: {}", e));
                    self.filtered_dataset = Some(dataset.clone());
                }
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
                    
                    if let Some(ref dataset) = self.dataset {
                        egui::ScrollArea::vertical()
                            .max_height(300.0)
                            .show(ui, |ui| {
                                let column_names: Vec<String> = dataset.get_column_names()
                                    .iter()
                                    .map(|s| s.to_string())
                                    .collect();
                                
                                for column_name in column_names {
                                    let mut is_visible = !self.hidden_columns.contains(&column_name);
                                    
                                    if ui.checkbox(&mut is_visible, &column_name).changed() {
                                        if is_visible {
                                            self.hidden_columns.remove(&column_name);
                                        } else {
                                            self.hidden_columns.insert(column_name);
                                        }
                                    }
                                }
                            });
                        
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("Show All").clicked() {
                                self.hidden_columns.clear();
                            }
                            if ui.button("Hide All").clicked() {
                                let column_names: Vec<String> = dataset.get_column_names()
                                    .iter()
                                    .map(|s| s.to_string())
                                    .collect();
                                for col in column_names {
                                    self.hidden_columns.insert(col);
                                }
                            }
                        });
                    }
                    
                    if ui.button("Close").clicked() {
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

    fn render_dataset_table(&mut self, ui: &mut egui::Ui) {
        if let Some(dataset) = self.filtered_dataset.clone() {
            let available_height = ui.available_height() - 100.0;
            
            egui::ScrollArea::both()
                .max_height(available_height)
                .show(ui, |ui| {
                    // Filter inputs
                    ui.horizontal(|ui| {
                        ui.label("Filters:");
                        if ui.button("Columns...").clicked() {
                            self.show_column_selector = true;
                        }
                    });
                    
                    let visible_columns = self.get_visible_columns(&dataset);
                    
                    ui.horizontal_wrapped(|ui| {
                        for column_name_str in &visible_columns {
                            ui.group(|ui| {
                                ui.vertical(|ui| {
                                    ui.strong(column_name_str);
                                    let filter_text = self.column_filters.get_mut(column_name_str).unwrap();
                                    let response = ui.text_edit_singleline(filter_text);
                                    
                                    if response.changed() {
                                        self.apply_filters();
                                    }
                                });
                            });
                        }
                    });
                    
                    ui.separator();
                    
                    // Data table using TableBuilder
                    use egui_extras::{Column, TableBuilder};
                    
                    let num_columns = visible_columns.len();
                    let num_rows = dataset.height().min(1000);
                    
                    if num_columns > 0 {
                        TableBuilder::new(ui)
                            .striped(true)
                            .resizable(true)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .columns(Column::auto().at_least(100.0), num_columns)
                            .header(25.0, |mut header| {
                                for column_name in &visible_columns {
                                    header.col(|ui| {
                                        ui.strong(column_name);
                                    });
                                }
                            })
                            .body(|mut body| {
                                for row_idx in 0..num_rows {
                                    body.row(20.0, |mut row| {
                                        for column_name in &visible_columns {
                                            row.col(|ui| {
                                                if let Ok(column) = dataset.column(column_name) {
                                                    let cell_value = format_cell_value(column, row_idx);
                                                    ui.label(cell_value);
                                                }
                                            });
                                        }
                                    });
                                }
                            });
                    } else {
                        ui.label("No visible columns. Use 'Columns...' to show some columns.");
                    }
                });
        }
    }
}

impl eframe::App for SigViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                
                ui.menu_button("Data View", |ui| {
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
                });
                
                ui.menu_button("View", |ui| {
                    if ui.button("Clear Filters").clicked() {
                        // ... existing code
                        ui.close();
                    }
                    if ui.button("Column Visibility...").clicked() {
                        self.show_column_selector = true;
                        ui.close();
                    }
                    
                    // Add theme toggle
                    ui.separator();
                    if ui.checkbox(&mut self.use_dark_theme, "Dark Theme").changed() {
                        if self.use_dark_theme {
                            ctx.set_visuals(egui::Visuals::dark());
                        } else {
                            ctx.set_visuals(egui::Visuals::light());
                        }
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