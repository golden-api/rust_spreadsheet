use crate::utils_gui::{col_label, parse_cell_name};
use crate::{Cell, FormulaType, STATUS, STATUS_CODE, Valtype, dependency, parser};

use eframe::{
    egui,
    egui::{Color32, Stroke, Vec2},
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
            header_bg: Color32::from_rgb(60, 63, 100),
            header_text: Color32::from_rgb(220, 220, 220),
            cell_bg_even: Color32::from_rgb(65, 50, 85),
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
    selected: Option<(usize, usize)>,
    formula_input: String,
    editing_cell: bool,
    style: SpreadsheetStyle,
    status_message: String,
    start_row: usize,
    start_col: usize,
    scroll_to_cell: String,
    should_reset_scroll: bool,
    focus_on_scroll_to: bool,
    request_formula_focus: bool,
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
            selected: Some((0, 0)),
            formula_input: String::new(),
            editing_cell: false,
            style: SpreadsheetStyle::default(),
            status_message: String::new(),
            start_row,
            start_col,
            scroll_to_cell: "..".to_string(),
            should_reset_scroll: false,
            focus_on_scroll_to: false,
            request_formula_focus: false,
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
        // Save these values before starting any mutable borrow.
        let total_rows = self.sheet.len();
        let total_cols = self.sheet[0].len();

        // Enclose the mutable operations in a block to end the borrow.
        if let Some((r, c)) = self.selected {
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
    }

    // Render the formula input bar
    fn render_formula_bar(&mut self, ui: &mut egui::Ui) {
        egui::Frame::NONE
            .fill(self.style.header_bg)
            .inner_margin(egui::Vec2::new(8.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Choose the hint text based on whether a cell is selected.
                    let hint = if self.selected.is_some() {
                        "Enter formula or value..."
                    } else {
                        "Enter command..."
                    };

                    // Add the text edit control and capture its response
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.formula_input)
                            .id_salt("command bar")
                            .hint_text(hint)
                            .desired_width(ui.available_width() - 120.0)
                            .font(egui::TextStyle::Monospace)
                            .text_color(self.style.header_text),
                    );

                    // Update focus state when this control gains focus
                    if self.request_formula_focus {
                        response.request_focus();
                        self.request_formula_focus = false;
                    }
                    // Only process Enter key when formula bar has focus
                    let process_formula = ui
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
                        || (response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));

                    if process_formula {
                        if self.selected.is_some() {
                            self.update_selected_cell();
                            self.editing_cell = false;
                        } else {
                            // Here you can add your command processing logic later.
                            self.status_message =
                                format!("Command received: {}", self.formula_input);
                            self.formula_input.clear();
                        }
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

            // Store the response from the text field
            let text_response = ui.add(
                egui::TextEdit::singleline(&mut self.scroll_to_cell)
                    .hint_text("e.g. AB78")
                    .desired_width(80.0)
                    .font(egui::TextStyle::Monospace)
                    .text_color(self.style.header_text),
            );

            if text_response.gained_focus() {
                self.focus_on_scroll_to = true;
            }

            // Separate check for Enter key when text field has focus
            let enter_pressed =
                self.focus_on_scroll_to && ui.input(|i| i.key_pressed(egui::Key::Enter));

            let button_clicked = ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("Go")
                            .size(self.style.font_size)
                            .color(self.style.header_text),
                    )
                    .fill(self.style.selected_cell_bg)
                    .min_size(egui::Vec2::new(60.0, 25.0)),
                )
                .clicked();

            // Process if either Enter was pressed or button was clicked
            if enter_pressed || button_clicked {
                self.process_scroll_to_cell();
            }
        });
    }

    fn process_scroll_to_cell(&mut self) {
        if let Some((target_row, target_col)) = parse_cell_name(&self.scroll_to_cell) {
            self.start_row = target_row;
            self.start_col = target_col;
            self.should_reset_scroll = true;
            self.status_message = format!(
                "Scrolled to cell {}{}",
                col_label(target_col),
                target_row + 1
            );
        } else {
            self.status_message = "Invalid cell name".to_string();
        }
        self.scroll_to_cell = String::new();
    }

    // Render a single cell
    fn render_cell(
        &mut self,
        ui: &mut egui::Ui,
        row: usize,
        col: usize,
        rect: egui::Rect,
    ) -> Option<(usize, usize)> {
        let is_selected = self.selected == Some((row, col));
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
                if self.selected == Some((row, col)) {
                    self.editing_cell = true;
                }
            }
        }

        new_selection
    }

    // Render an editable cell (when editing)
    fn render_editable_cell(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
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

    // // Render the main spreadsheet grid
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
        };
        let mut scroll_offset = egui::Vec2::ZERO;
        scroll_area.show(ui, |ui| {
            let (virtual_rect, _) = ui.allocate_exact_size(virtual_size, egui::Sense::hover());
            scroll_offset = ui.clip_rect().min - virtual_rect.min;

            // Calculate visible indices
            let render_start_col =
                self.start_col + (scroll_offset.x / cell_size.x).floor() as usize;
            let render_start_row =
                self.start_row + (scroll_offset.y / cell_size.y).floor() as usize;
            let visible_cols = (((ui.available_rect_before_wrap().size().x - row_label_width)
                / cell_size.x)
                .ceil() as usize)
                .max(1)
                + 1;
            let visible_rows = total_rows.min(33);

            // Render data cells
            for i in render_start_row..(render_start_row + visible_rows).min(total_rows) {
                for j in render_start_col..(render_start_col + visible_cols).min(total_cols) {
                    let x = virtual_rect.min.x
                        + row_label_width
                        + (j - self.start_col) as f32 * cell_size.x;
                    let y = virtual_rect.min.y
                        + header_height
                        + (i - self.start_row) as f32 * cell_size.y;
                    let cell_rect = egui::Rect::from_min_size(egui::pos2(x, y), cell_size);

                    if let Some(selection) = self.render_cell(ui, i, j, cell_rect) {
                        new_selection = Some(selection);
                    }
                }
            }
        });
        let painter = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("pinned_headers"),
        ));

        // The reference origin from where we want to draw headers.
        // For a minimal approach, you can use `ui.min_rect().min`.
        let base_x = ui.min_rect().min.x;
        let base_y = ui.min_rect().min.y;

        // --- Column Headers (pinned vertically, scrolled horizontally) ---
        for col_idx in self.start_col..total_cols {
            // shift horizontally with the cells by subtracting scroll_offset.x
            let header_x = base_x - scroll_offset.x
                + (col_idx - self.start_col) as f32 * cell_size.x
                + row_label_width; // if you want them to start after the row-label region

            let header_rect = egui::Rect::from_min_size(
                egui::pos2(header_x.max(base_x), base_y),
                egui::vec2(cell_size.x, header_height),
            );
            // draw background
            painter.rect_filled(header_rect, 0.0, self.style.header_bg);
            // draw label text
            painter.text(
                header_rect.center(),
                egui::Align2::CENTER_CENTER,
                col_label(col_idx),
                egui::FontId::monospace(self.style.font_size),
                self.style.header_text,
            );

            // stroke border if needed
            use egui::epaint::StrokeKind;
            painter.rect_stroke(header_rect, 0.0, self.style.grid_line, StrokeKind::Middle);
        }
        // --- Row Labels (pinned horizontally, scrolled vertically) ---
        for row_idx in self.start_row..total_rows {
            // shift vertically with the cells by subtracting scroll_offset.y
            let header_y = base_y - scroll_offset.y
                + (row_idx - self.start_row) as f32 * cell_size.y
                + header_height; // if you want them to start after the column-header region
            let row_rect = egui::Rect::from_min_size(
                egui::pos2(base_x, header_y.max(base_y)),
                egui::vec2(row_label_width, cell_size.y),
            );
            painter.rect_filled(row_rect, 0.0, self.style.header_bg);
            painter.text(
                row_rect.center(),
                egui::Align2::CENTER_CENTER,
                (row_idx + 1).to_string(),
                egui::FontId::monospace(self.style.font_size),
                self.style.header_text,
            );
            use egui::epaint::StrokeKind;
            painter.rect_stroke(row_rect, 0.0, self.style.grid_line, StrokeKind::Inside);
        }

        // --- Corner Cell (optional) ---
        let corner_rect = egui::Rect::from_min_size(
            egui::pos2(base_x, base_y),
            egui::vec2(row_label_width, header_height),
        );
        use egui::epaint::StrokeKind;
        painter.rect_filled(corner_rect, 0.0, self.style.header_bg);
        painter.rect_stroke(corner_rect, 0.0, self.style.grid_line, StrokeKind::Outside);

        // Reset the flag after rendering to allow normal scrolling
        self.should_reset_scroll = false;
        new_selection
    }

    // Display information about the selected cell
    fn render_selected_cell_info(&self, ui: &mut egui::Ui) {
        ui.add_space(5.0);
        if let Some((row, col)) = self.selected {
            ui.label(
                egui::RichText::new(format!("Selected Cell: {}{}", col_label(col), row + 1))
                    .size(self.style.font_size)
                    .color(self.style.header_text),
            );
        }
    }

    // Handle keyboard events like Escape key
    fn handle_keyboard_events(&mut self, ctx: &egui::Context) {
        ctx.input(|input| {
            if input.key_pressed(egui::Key::ArrowDown) {
                if let Some((row, col)) = self.selected {
                    if row + 1 < self.sheet.len() {
                        self.selected = Some((row + 1, col));
                        self.should_reset_scroll = true; // Optional: scroll into view
                    }
                }
            } else if input.key_pressed(egui::Key::ArrowUp) {
                if let Some((row, col)) = self.selected {
                    if row > 0 {
                        self.selected = Some((row - 1, col));
                        self.should_reset_scroll = true;
                    }
                }
            } else if input.key_pressed(egui::Key::ArrowRight) {
                if let Some((row, col)) = self.selected {
                    if col + 1 < self.sheet[0].len() {
                        self.selected = Some((row, col + 1));
                        self.should_reset_scroll = true;
                    }
                }
            } else if input.key_pressed(egui::Key::ArrowLeft) {
                if let Some((row, col)) = self.selected {
                    if col > 0 {
                        self.selected = Some((row, col - 1));
                        self.should_reset_scroll = true;
                    }
                }
            } else if input.key_pressed(egui::Key::Escape) {
                if self.editing_cell {
                    self.editing_cell = false;
                    if let Some((row, col)) = self.selected {
                        self.formula_input = self.get_cell_formula(row, col);
                    }
                } else {
                    self.selected = None;
                    self.formula_input.clear();
                    self.status_message = "Selection cleared, command mode".to_string();
                    self.request_formula_focus = true;
                }
            }
        });
    }

    // Handle cell selection changes
    fn handle_selection_change(&mut self, new_selection: Option<(usize, usize)>) {
        if let Some((i, j)) = new_selection {
            // Only update when a new selection is made.
            self.selected = Some((i, j));
            self.formula_input = self.get_cell_formula(i, j);
            self.status_message = format!("Selected cell {}{}", col_label(j), i + 1);
        }
    }
}

impl eframe::App for SpreadsheetApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());

        let mut new_selection = None;
        egui::TopBottomPanel::top("formula_panel").show(ctx, |ui| {
            self.render_formula_bar(ui);
            self.render_scroll_to_cell(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(selection) = self.render_spreadsheet_grid(ui) {
                new_selection = Some(selection);
            }
            self.render_selected_cell_info(ui);
        });

        self.handle_selection_change(new_selection);
        self.handle_keyboard_events(ctx);
    }
}
