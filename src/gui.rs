use crate::dependency::update_cell;
use crate::{Cell, FormulaType, Valtype};
use eframe::{
    egui,
    egui::{Button, CentralPanel, ScrollArea, TextEdit},
};
use std::collections::HashSet;

pub struct SpreadsheetApp {
    sheet: Vec<Vec<Cell>>,
    selected: (usize, usize),
    formula_input: String,
}

impl SpreadsheetApp {
    pub fn new() -> Self {
        let rows = 30;
        let cols = 30;
        let sheet = vec![
            vec![
                Cell {
                    value: Valtype::Int(0),
                    value2: Valtype::Int(0),
                    formula: None,
                    op_code: None,
                    cell1: None,
                    cell2: None,
                    dependents: HashSet::new(),
                };
                cols
            ];
            rows
        ];
        Self {
            sheet,
            selected: (0, 0),
            formula_input: String::new(),
        }
    }
}

impl eframe::App for SpreadsheetApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        use crate::parser::detect_formula;
        use crate::parser::recalc;
        CentralPanel::default().show(ctx, |ui| {
            // Top formula input
            ui.horizontal(|ui| {
                ui.add(TextEdit::singleline(&mut self.formula_input).hint_text("Enter formula..."));
                if ui.button("Update Cell").clicked() {
                    let (r, c) = self.selected;
                    let total_rows = self.sheet.len();
                    let total_cols = self.sheet[0].len();

                    // Set formula into the cell
                    let cell = &mut self.sheet[r][c];
                    cell.reset(); // optional, but safe
                    detect_formula(cell, &self.formula_input); // parse and update cell

                    // Recalculate all dependent cells
                    recalc(&mut self.sheet, total_rows, total_cols, r, c);
                }
            });

            // Scrollable spreadsheet grid
            ScrollArea::both().auto_shrink([false; 2]).show(ui, |ui| {
                for (i, row) in self.sheet.iter().enumerate() {
                    ui.horizontal(|ui| {
                        for (j, cell) in row.iter().enumerate() {
                            let text = match &cell.value {
                                Valtype::Int(n) => n.to_string(),
                                Valtype::Str(s) => s.clone(),
                            };
                            let is_selected = self.selected == (i, j);
                            let mut btn = Button::new(text).min_size(egui::vec2(80.0, 30.0));
                            if is_selected {
                                btn = btn.fill(egui::Color32::LIGHT_GREEN);
                            }
                            if ui.add(btn).clicked() {
                                self.selected = (i, j);
                                self.formula_input = cell
                                    .formula
                                    .as_ref()
                                    .map(|f| format!("{:?}", f))
                                    .unwrap_or_default();
                            }
                        }
                    });
                }
            });
        });
    }
}
