use crate::{Cell, FormulaType, STATUS, STATUS_CODE, Valtype, dependency, parser};
use crate::utils_gui::{col_label, parse_cell_name};

use eframe::{
    egui,
    egui::{Color32,Stroke, Vec2},
};
use std::collections::HashSet;

// Define your styling configuration.
pub struct SpreadsheetStyle {
    header_bg: Color32,
    header_text: Color32,
    cell_bg_even: Color32,
    cell_bg_odd: Color32,
    cell_text: Color32,
    selected_cell_bg: Color32,
    selected_cell_text: Color32,
    grid_line: Stroke,
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
            selected_cell_bg: Color32::from_rgb(120, 120, 180),
            selected_cell_text: Color32::WHITE,
            grid_line: Stroke::new(1.0, Color32::from_rgb(70, 70, 70)),
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
    scroll_to_cell: String,
    should_reset_scroll: bool
}

impl SpreadsheetApp {
    pub fn new(rows: usize, cols: usize, start_row: usize, start_col: usize) -> Self {
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
            start_row,
            start_col,
            scroll_to_cell: "A1".to_string(),
            should_reset_scroll: false
        }
    }

    // Helper: Extract formula from cell
    fn get_cell_formula(&self, row: usize, col: usize) -> String {
        let cell = &self.sheet[row][col];
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
        String::new()
    }

    // Update the value of the currently selected cell
    fn update_selected_cell(&mut self) {
        let (r, c) = self.selected;
        // Save these values before starting any mutable borrow.
        let total_rows = self.sheet.len();
        let total_cols = self.sheet[0].len();

        // Enclose the mutable operations in a block to end the borrow.
        let old_cell = self.sheet[r][c].my_clone();
        parser::detect_formula(&mut self.sheet[r][c], &self.formula_input);
        dependency::update_cell(&mut self.sheet, total_rows, total_cols, r, c, old_cell);
        if unsafe { STATUS_CODE } == 0 {
            parser::recalc(&mut self.sheet, total_rows, total_cols, r, c);
        }

        self.status_message = match unsafe { STATUS_CODE } {
            0 => format!("Updated cell {}{}", col_label(c), r + 1),
            code => format!("{}", STATUS[code]),
        };
        unsafe {
            STATUS_CODE = 0;
        }
    }

    // Render the formula input bar
    fn render_formula_bar(&mut self, ui: &mut egui::Ui) {
        egui::Frame::NONE
            .fill(self.style.header_bg)
            .inner_margin(5.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.formula_input)
                            .hint_text("Enter formula or value...")
                            .desired_width(ui.available_width() - 120.0)
                            .font(egui::TextStyle::Monospace)
                            .text_color(self.style.header_text),
                    );
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Update Cell")
                                    .size(self.style.font_size)
                                    .color(self.style.header_text),
                            )
                            .fill(self.style.selected_cell_bg)
                            .min_size(egui::Vec2::new(100.0, 25.0)),
                        )
                        .clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        self.update_selected_cell();
                        self.editing_cell = false;
                    }
                });
                if !self.status_message.is_empty() {
                    ui.label(
                        egui::RichText::new(&self.status_message)
                            .size(self.style.font_size - 2.0)
                            .color(self.style.header_text),
                    );
                }
            });
    }

    // Render the scroll-to-cell feature
    fn render_scroll_to_cell(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Scroll to:")
                    .size(self.style.font_size)
                    .color(self.style.header_text),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.scroll_to_cell)
                    .hint_text("e.g. AB78")
                    .desired_width(80.0)
                    .font(egui::TextStyle::Monospace)
                    .text_color(self.style.header_text),
            );
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("Go")
                            .size(self.style.font_size)
                            .color(self.style.header_text),
                    )
                    .fill(self.style.selected_cell_bg)
                    .min_size(egui::Vec2::new(60.0, 25.0)),
                )
                .clicked()
            {
                self.process_scroll_to_cell();
            }
        });
    }

    // Process the scroll-to-cell request
    fn process_scroll_to_cell(&mut self) {
        if let Some((target_row, target_col)) = parse_cell_name(&self.scroll_to_cell) {
            self.start_row = target_row;
            self.start_col = target_col;
            self.should_reset_scroll = true; // Trigger scroll reset
            self.status_message = format!(
                "Scrolled to cell {}{}",
                col_label(target_col),
                target_row + 1
            );
        } else {
            self.status_message = "Invalid cell name".to_string();
        }
    }

    // Render a column header
    fn render_column_header(&self, ui: &mut egui::Ui, col_idx: usize, x: f32, y: f32) {
        let header_rect = egui::Rect::from_min_size(
            egui::pos2(x, y), 
            self.style.cell_size
        );
        
        ui.put(
            header_rect,
            egui::Label::new(
                egui::RichText::new(col_label(col_idx))
                    .size(self.style.font_size)
                    .color(self.style.header_text),
            ),
        );
    }

    // Render a row header
    fn render_row_header(&self, ui: &mut egui::Ui, row_idx: usize, x: f32, y: f32, width: f32) {
        let row_label_rect = egui::Rect::from_min_size(
            egui::pos2(x, y),
            egui::vec2(width, self.style.cell_size.y),
        );
        
        ui.put(
            row_label_rect,
            egui::Label::new(
                egui::RichText::new((row_idx + 1).to_string())
                    .size(self.style.font_size)
                    .color(self.style.header_text),
            ),
        );
    }

    // Render a single cell
    fn render_cell(&mut self, ui: &mut egui::Ui, row: usize, col: usize, rect: egui::Rect) -> Option<(usize, usize)> {
        let is_selected = self.selected == (row, col);
        let mut new_selection = None;

        if is_selected && self.editing_cell {
            self.render_editable_cell(ui, rect);
        } else {
            let text = match &self.sheet[row][col].value {
                Valtype::Int(n) => n.to_string(),
                Valtype::Str(s) => s.clone(),
            };
            
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
            
            // Render button
            ui.put(
                rect,
                egui::Button::new(
                    egui::RichText::new(text)
                        .size(self.style.font_size)
                        .color(text_color),
                )
                .fill(bg_color)
                .stroke(self.style.grid_line),
            );
            
            // Handling interactions
            let response = ui.interact(
                rect,
                ui.make_persistent_id((row, col)),
                egui::Sense::click(),
            );
            
            if response.clicked() {
                new_selection = Some((row, col));
                if self.selected == (row, col) {
                    self.editing_cell = true;
                }
            }
        }
        
        new_selection
    }

    // Render an editable cell (when editing)
    fn render_editable_cell(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        ui.allocate_ui_at_rect(rect, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.formula_input)
                    .hint_text("Edit...")
                    .text_color(egui::Color32::WHITE)
                    .font(egui::TextStyle::Monospace),
            );
            
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.update_selected_cell();
                self.editing_cell = false;
            }
        });
    }

    // Render the main spreadsheet grid
    fn render_spreadsheet_grid(&mut self, ui: &mut egui::Ui) -> Option<(usize, usize)> {
        let mut new_selection = None;
        let cell_size = self.style.cell_size;
        let row_label_width = 30.0;
        let header_height = cell_size.y;
        
        // Calculate total cols and rows to render
        let total_cols = self.sheet[0].len().min(self.start_col + 200);
        let total_rows = self.sheet.len().min(self.start_row + 100);
        
        // Virtual size calculation
        let virtual_width = row_label_width + (total_cols - self.start_col) as f32 * cell_size.x;
        let virtual_height = header_height + (total_rows - self.start_row) as f32 * cell_size.y;
        let virtual_size = egui::vec2(virtual_width, virtual_height);
        
        // Configure ScrollArea
        let mut scroll_area = egui::ScrollArea::both()
            .id_salt((self.start_row, self.start_col))
            .drag_to_scroll(true)
            .auto_shrink([false, false]);
        
        // Reset scroll offset only when needed
        if self.should_reset_scroll {
            scroll_area = scroll_area.scroll_offset(egui::Vec2::ZERO);
        }

        scroll_area.show(ui, |ui| {
            let (virtual_rect, _) = ui.allocate_exact_size(virtual_size, egui::Sense::hover());
            let scroll_offset = ui.clip_rect().min - virtual_rect.min;
            
            // Calculate visible indices
            let render_start_col = self.start_col + (scroll_offset.x / cell_size.x).floor() as usize;
            let render_start_row = self.start_row + (scroll_offset.y / cell_size.y).floor() as usize;
            let visible_cols = (((ui.available_rect_before_wrap().size().x - row_label_width) / cell_size.x).ceil() as usize).max(1) + 1;
            let visible_rows = total_rows.min(33);
            
            // Render column headers
            for j in render_start_col..(render_start_col + visible_cols).min(total_cols) {
                let x = virtual_rect.min.x + row_label_width + (j - self.start_col) as f32 * cell_size.x;
                self.render_column_header(ui, j, x, virtual_rect.min.y);
            }
            
            // Render row labels
            for i in render_start_row..(render_start_row + visible_rows).min(total_rows) {
                let y = virtual_rect.min.y + header_height + (i - self.start_row) as f32 * cell_size.y;
                self.render_row_header(ui, i, virtual_rect.min.x, y, row_label_width);
            }
            
            // Render data cells
            for i in render_start_row..(render_start_row + visible_rows).min(total_rows) {
                for j in render_start_col..(render_start_col + visible_cols).min(total_cols) {
                    let x = virtual_rect.min.x + row_label_width + (j - self.start_col) as f32 * cell_size.x;
                    let y = virtual_rect.min.y + header_height + (i - self.start_row) as f32 * cell_size.y;
                    let cell_rect = egui::Rect::from_min_size(egui::pos2(x, y), cell_size);
                    
                    if let Some(selection) = self.render_cell(ui, i, j, cell_rect) {
                        new_selection = Some(selection);
                    }
                }
            }
        });
        
        // Reset the flag after rendering to allow normal scrolling
        self.should_reset_scroll = false;
        
        new_selection
    }

    // Display information about the selected cell
    fn render_selected_cell_info(&self, ui: &mut egui::Ui) {
        ui.add_space(5.0);
        let (row, col) = self.selected;
        ui.label(
            egui::RichText::new(format!("Selected Cell: {}{}", col_label(col), row + 1))
                .size(self.style.font_size)
                .color(self.style.header_text),
        );
    }

    // Handle keyboard events like Escape key
    fn handle_keyboard_events(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.editing_cell {
                self.editing_cell = false;
                self.formula_input = self.get_cell_formula(self.selected.0, self.selected.1);
            } else {
                self.selected = (0, 0);
                self.formula_input.clear();
                self.status_message = "Selection cleared".to_string();
            }
        }
    }

    // Handle cell selection changes
    fn handle_selection_change(&mut self, new_selection: Option<(usize, usize)>) {
        if let Some((i, j)) = new_selection {
            self.selected = (i, j);
            self.formula_input = self.get_cell_formula(i, j);
            self.status_message = format!("Selected cell {}{}", col_label(j), i + 1);
        }
    }
}

impl eframe::App for SpreadsheetApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());
        
        let mut new_selection = None;
        
        egui::CentralPanel::default().show(ctx, |ui| {
            // Render formula bar
            self.render_formula_bar(ui);
            
            // Render scroll-to-cell feature
            self.render_scroll_to_cell(ui);
            
            // Render main spreadsheet grid
            if let Some(selection) = self.render_spreadsheet_grid(ui) {
                new_selection = Some(selection);
            }
            
            // Render selected cell info
            self.render_selected_cell_info(ui);
        });
        
        // Handle selection changes
        self.handle_selection_change(new_selection);
        
        // Handle keyboard events
        self.handle_keyboard_events(ctx);
    }
}
