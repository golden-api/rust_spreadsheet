use crate::{Cell, FormulaType, Valtype, dependency, parser, STATUS_CODE, STATUS};
use eframe::{
    egui,
    egui::{Button, CentralPanel, Color32, Frame, RichText, ScrollArea, Stroke, TextEdit, Vec2},
};
use std::collections::HashSet;

// Define a struct to hold the styling configuration
pub struct SpreadsheetStyle {
    header_bg: Color32,
    header_text: Color32,
    cell_bg_even: Color32,
    cell_bg_odd: Color32,
    cell_text: Color32,
    selected_cell_bg: Color32,
    selected_cell_text: Color32,
    grid_line: Stroke,
    cell_padding: Vec2,
    cell_size: Vec2,
    font_size: f32,
}

impl Default for SpreadsheetStyle {
    fn default() -> Self {
        Self {
            header_bg: Color32::from_rgb(60, 63, 65),
            header_text: Color32::from_rgb(220, 220, 220),
            cell_bg_even: Color32::from_rgb(50, 50, 50),
            cell_bg_odd: Color32::from_rgb(45, 45, 45),
            cell_text: Color32::LIGHT_GRAY,
            selected_cell_bg: Color32::from_rgb(100, 180, 100),
            selected_cell_text: Color32::BLACK,
            grid_line: Stroke::new(1.0, Color32::from_rgb(70, 70, 70)),
            cell_padding: Vec2::new(8.0, 4.0),
            cell_size: Vec2::new(60.0, 25.0),
            font_size: 14.0,
        }
    }
}

pub struct SpreadsheetApp {
    sheet: Vec<Vec<Cell>>,
    selected: (usize, usize),
    formula_input: String,
    editing_cell: bool,
    style: SpreadsheetStyle,
    status_message: String,
    start_row: usize,
    start_col: usize,
}

impl SpreadsheetApp {
    pub fn new() -> Self {
        let rows = 30;
        let cols = 30;
        // Initialize the sheet with default cells.
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
            editing_cell: false,
            style: SpreadsheetStyle::default(),
            status_message: String::new(),
            start_row: 0,
            start_col: 0,
        }
    }

    // Method to customize the style
    pub fn with_style(mut self, style: SpreadsheetStyle) -> Self {
        self.style = style;
        self
    }

    fn get_cell_formula(&self, row: usize, col: usize) -> String {
        let cell = &self.sheet[row][col];

        // Return the original formula string if it exists
        if let Some(formula_type) = &cell.formula {
            match formula_type {
                FormulaType::Const => {
                    if let Valtype::Int(val) = cell.value {
                        return val.to_string();
                    }
                }
                FormulaType::Ref => {
                    if let Some(ref1) = &cell.cell1 {
                        return ref1.clone();
                    }
                }
                FormulaType::CoC => {
                    if let (Valtype::Int(val1), Valtype::Int(val2), Some(op)) =
                        (&cell.value, &cell.value2, &cell.op_code)
                    {
                        return format!("{}{}{}", val1, op, val2);
                    }
                }
                FormulaType::CoR => {
                    if let (Valtype::Int(val), Some(ref2), Some(op)) =
                        (&cell.value2, &cell.cell2, &cell.op_code)
                    {
                        return format!("{}{}{}", val, op, ref2);
                    }
                }
                FormulaType::RoC => {
                    if let (Some(ref1), Valtype::Int(val), Some(op)) =
                        (&cell.cell1, &cell.value2, &cell.op_code)
                    {
                        return format!("{}{}{}", ref1, op, val);
                    }
                }
                FormulaType::RoR => {
                    if let (Some(ref1), Some(ref2), Some(op)) =
                        (&cell.cell1, &cell.cell2, &cell.op_code)
                    {
                        return format!("{}{}{}", ref1, op, ref2);
                    }
                }
                FormulaType::Range => {
                    if let (Some(ref1), Some(ref2), Valtype::Str(func)) =
                        (&cell.cell1, &cell.cell2, &cell.value2)
                    {
                        return format!("{}({}:{})", func, ref1, ref2);
                    }
                }
                FormulaType::SleepC => {
                    if let Valtype::Int(val) = cell.value {
                        return format!("SLEEP({})", val);
                    }
                }
                FormulaType::SleepR => {
                    if let Some(ref1) = &cell.cell1 {
                        return format!("SLEEP({})", ref1);
                    }
                }
                _ => {}
            }
        }
        
        // Default to empty string if no formula or unable to reconstruct
        String::new()
    }
    
    fn update_selected_cell(&mut self) {
        let (r, c) = self.selected;
        let total_rows = self.sheet.len();
        let total_cols = self.sheet[0].len();
        
        // Reset the cell before applying new data.
        let old_cell = self.sheet[r][c].my_clone();
        parser::detect_formula(&mut self.sheet[r][c], &self.formula_input);
        dependency::update_cell(&mut self.sheet, total_rows, total_cols, r, c, old_cell);
        
        if unsafe { STATUS_CODE } == 0 {
            parser::recalc(&mut self.sheet, total_rows, total_cols, r, c);
        }
        
        // Update status message based on status code
        self.status_message = match unsafe { STATUS_CODE } {
            0 => format!("Updated cell {}{}", (b'A' + (c as u8)) as char, r + 1),
            code => format!("{}", unsafe { STATUS[code] })
        };
        
        unsafe { STATUS_CODE = 0; }
    }
}

impl eframe::App for SpreadsheetApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Set a dark theme
        ctx.set_visuals(egui::Visuals::dark());

        // Track if we need to update the formula input
        let mut new_selection = None;

        CentralPanel::default().show(ctx, |ui| {
            // Formula input area with improved styling
            Frame::none()
                .fill(self.style.header_bg)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(
                            TextEdit::singleline(&mut self.formula_input)
                                .hint_text("Enter formula or value...")
                                .desired_width(ui.available_width() - 120.0)
                                .font(egui::TextStyle::Monospace)
                                .text_color(self.style.header_text),
                        );

                        // Auto-update when Enter is pressed
                        let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if ui
                            .add(
                                Button::new(
                                    RichText::new("Update Cell")
                                        .size(self.style.font_size)
                                        .color(self.style.header_text),
                                )
                                .fill(self.style.selected_cell_bg)
                                .min_size(Vec2::new(100.0, 30.0)),
                            )
                            .clicked()
                            || enter_pressed
                        {
                            self.update_selected_cell();
                            self.editing_cell = false;
                        }
                    });

                    // Navigation controls
                    ui.horizontal(|ui| {
                        if ui.button("↑").clicked() {
                            crate::scrolling::w(&mut self.start_row);
                        }
                        if ui.button("↓").clicked() {
                            crate::scrolling::s(&mut self.start_row, self.sheet.len());
                        }
                        if ui.button("←").clicked() {
                            crate::scrolling::a(&mut self.start_col);
                        }
                        if ui.button("→").clicked() {
                            crate::scrolling::d(&mut self.start_col, self.sheet[0].len());
                        }
                    });

                    // Status message
                    if !self.status_message.is_empty() {
                        ui.add_space(5.0);
                        ui.label(
                            RichText::new(&self.status_message)
                                .size(self.style.font_size - 2.0)
                                .color(self.style.header_text),
                        );
                    }
                });

            // Display the spreadsheet grid in a scrollable area with improved styling
            ScrollArea::both().auto_shrink([false; 2]).show(ui, |ui| {
                // Add column headers (A, B, C, ...)
                ui.horizontal(|ui| {
                    // Empty cell for row/column intersection
                    ui.add_sized(
                        Vec2::new(30.0, self.style.cell_size.y),
                        egui::Label::new(
                            RichText::new("")
                                .color(self.style.header_text)
                                .size(self.style.font_size),
                        ),
                    );

                    // Only render visible columns
                    let visible_cols = 20; // Adjust based on your UI size
                    for j in 0..visible_cols {
                        let col = self.start_col + j;
                        if col < self.sheet[0].len() {
                            let col_name = (b'A' + (col as u8)) as char;
                            ui.add_sized(
                                self.style.cell_size,
                                egui::Label::new(
                                    RichText::new(col_name.to_string())
                                        .color(self.style.header_text)
                                        .size(self.style.font_size),
                                ),
                            );
                        }
                    }
                });

                // Only render visible rows
                let visible_rows = 20; // Adjust based on your UI size
                for i in 0..visible_rows {
                    let row = self.start_row + i;
                    if row < self.sheet.len() {
                        ui.horizontal(|ui| {
                            // Add row numbers with styling
                            ui.add_sized(
                                Vec2::new(30.0, self.style.cell_size.y),
                                egui::Label::new(
                                    RichText::new((row + 1).to_string())
                                        .color(self.style.header_text)
                                        .size(self.style.font_size),
                                ),
                            );

                            // Only render visible columns
                            let visible_cols = 20; // Adjust based on your UI size
                            for j in 0..visible_cols {
                                let col = self.start_col + j;
                                if col < self.sheet[row].len() {
                                    let is_selected = self.selected == (row, col);
                                    
                                    if is_selected && self.editing_cell {
                                        // Show text edit when cell is selected and in editing mode
                                        let response = ui.add_sized(
                                            self.style.cell_size,
                                            TextEdit::singleline(&mut self.formula_input)
                                                .hint_text("Edit...")
                                                .text_color(self.style.selected_cell_text)
                                                .font(egui::TextStyle::Monospace)
                                        );
                                        
                                        // Handle Enter key to confirm edit
                                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                            self.update_selected_cell();
                                            self.editing_cell = false;
                                        }
                                    } else {
                                        // Render the cell value as text.
                                        let text = match &self.sheet[row][col].value {
                                            Valtype::Int(n) => n.to_string(),
                                            Valtype::Str(s) => s.clone(),
                                        };

                                        // Alternate row colors for better readability
                                        let bg_color = if is_selected {
                                            self.style.selected_cell_bg
                                        } else if row % 2 == 0 {
                                            self.style.cell_bg_even
                                        } else {
                                            self.style.cell_bg_odd
                                        };

                                        let text_color = if is_selected {
                                            self.style.selected_cell_text
                                        } else {
                                            self.style.cell_text
                                        };

                                        let cell_response = ui.add_sized(
                                            self.style.cell_size,
                                            Button::new(
                                                RichText::new(text)
                                                    .size(self.style.font_size)
                                                    .color(text_color),
                                            )
                                            .fill(bg_color)
                                            .stroke(self.style.grid_line),
                                        );

                                        if cell_response.clicked() {
                                            // Store the new selection to process after the loop
                                            new_selection = Some((row, col));
                                            
                                            // Start editing on double-click
                                            if self.selected == (row, col) {
                                                self.editing_cell = true;
                                            }
                                        }
                                    }
                                }
                            }
                        });
                    }
                }
            });

            // Display current cell reference
            ui.add_space(5.0);
            let (row, col) = self.selected;
            let col_letter = (b'A' + (col as u8)) as char;
            ui.label(
                RichText::new(format!("Selected Cell: {}{}", col_letter, row + 1))
                    .size(self.style.font_size)
                    .color(self.style.header_text),
            );
        });

        // Process new selection outside of the closure
        if let Some((i, j)) = new_selection {
            self.selected = (i, j);
            self.formula_input = self.get_cell_formula(i, j);
            self.status_message = format!("Selected cell {}{}", (b'A' + (j as u8)) as char, i + 1);
        }
        
        // Handle Escape key to deselect cell or exit editing mode
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.editing_cell {
                self.editing_cell = false;
                self.formula_input = self.get_cell_formula(self.selected.0, self.selected.1);
            } else {
                // Reset selection
                self.selected = (0, 0);
                self.formula_input.clear();
                self.status_message = "Selection cleared".to_string();
            }
        }
    }
}
