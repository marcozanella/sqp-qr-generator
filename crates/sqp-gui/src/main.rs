mod qr;

use chrono::{Datelike, Local, Months};
use eframe::egui;
use sqp_core::{
    auto_batch, generate, GenerateRequest, GeneratedCode, InkType, SUPPORTED_COLOR_NAMES,
};
use std::collections::HashSet;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([720.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "SQP QR Generator",
        options,
        Box::new(|_cc| Box::new(App::default())),
    )
}

struct App {
    ink_type: InkType,
    color: String,
    volume_l: u8,
    expires_year: i32,
    expires_month: u8,
    batch: String,
    count: u8,
    results: Vec<GeneratedCode>,
    produced: HashSet<String>,
    error: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        let today = Local::now().date_naive();
        let exp = today.checked_add_months(Months::new(12)).unwrap();
        Self {
            ink_type: InkType::Kx2,
            color: "Cyan".to_string(),
            volume_l: 5,
            expires_year: exp.year(),
            expires_month: exp.month() as u8,
            batch: auto_batch(today),
            count: 1,
            results: Vec::new(),
            produced: HashSet::new(),
            error: None,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("form").min_width(280.0).show(ctx, |ui| {
            ui.heading("New codes");
            ui.add_space(8.0);

            ui.label("Ink type");
            egui::ComboBox::from_id_source("ink")
                .selected_text(match self.ink_type {
                    InkType::Kx2 => "KX2",
                    InkType::Sqsg3 => "SQSG3",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.ink_type, InkType::Kx2, "KX2");
                    ui.selectable_value(&mut self.ink_type, InkType::Sqsg3, "SQSG3");
                });

            ui.label("Color");
            egui::ComboBox::from_id_source("color")
                .selected_text(&self.color)
                .show_ui(ui, |ui| {
                    for name in SUPPORTED_COLOR_NAMES {
                        ui.selectable_value(&mut self.color, name.to_string(), name);
                    }
                });

            ui.label("Volume");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.volume_l, 1, "1 L");
                ui.selectable_value(&mut self.volume_l, 5, "5 L");
            });

            ui.label("Expiry (month / year)");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut self.expires_month).clamp_range(1..=12));
                ui.add(egui::DragValue::new(&mut self.expires_year).clamp_range(2007..=2070));
            });

            ui.label("Batch (9 chars, A-Z 0-9)");
            ui.text_edit_singleline(&mut self.batch);

            ui.label("How many codes (1-20)");
            ui.add(egui::DragValue::new(&mut self.count).clamp_range(1..=20));

            ui.add_space(12.0);
            if ui.button("Generate").clicked() {
                self.error = None;
                let req = GenerateRequest {
                    ink_type: self.ink_type,
                    color: self.color.clone(),
                    volume_l: self.volume_l,
                    expires_year: self.expires_year,
                    expires_month: self.expires_month,
                    batch: self.batch.clone(),
                    count: self.count,
                };
                match generate(&req, &mut self.produced) {
                    Ok(mut codes) => self.results.append(&mut codes),
                    Err(e) => self.error = Some(format!("{e:?}")),
                }
            }
            if let Some(err) = &self.error {
                ui.colored_label(egui::Color32::RED, err);
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Results");

            if !self.results.is_empty() && ui.button("Export all (PNGs + CSV)").clicked() {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    let mut csv =
                        String::from("code,ink_type,color,volume,expires,batch,number\n");
                    for g in &self.results {
                        let png = qr::qr_png_bytes(&g.code_undashed, 6);
                        let _ = std::fs::write(dir.join(format!("{}.png", g.code_undashed)), png);
                        csv.push_str(&format!(
                            "{},{},{},{},{:02}/{},{},{}\n",
                            g.code,
                            match self.ink_type {
                                InkType::Kx2 => "KX2",
                                InkType::Sqsg3 => "SQSG3",
                            },
                            self.color,
                            self.volume_l,
                            self.expires_month,
                            self.expires_year,
                            self.batch,
                            g.number
                        ));
                    }
                    let _ = std::fs::write(dir.join("codes.csv"), csv);
                }
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                for g in &self.results {
                    ui.horizontal(|ui| {
                        let (side, rgba) = qr::qr_rgba(&g.code_undashed, 4);
                        let image = egui::ColorImage::from_rgba_unmultiplied([side, side], &rgba);
                        let tex =
                            ui.ctx()
                                .load_texture(&g.code, image, egui::TextureOptions::NEAREST);
                        ui.image((tex.id(), egui::vec2(120.0, 120.0)));
                        ui.monospace(&g.code);
                        if ui.button("Copy").clicked() {
                            ui.output_mut(|o| o.copied_text = g.code.clone());
                        }
                        if ui.button("Save QR").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name(format!("{}.png", g.code_undashed))
                                .save_file()
                            {
                                let _ = std::fs::write(path, qr::qr_png_bytes(&g.code_undashed, 6));
                            }
                        }
                    });
                    ui.separator();
                }
            });
        });
    }
}
