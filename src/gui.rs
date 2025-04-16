#[cfg(feature = "gui")]
use crate::utils_gui::{col_label, parse_cell_name};
#[cfg(feature = "gui")]
use crate::{Cell, CellData, STATUS, STATUS_CODE, Valtype, parser};
#[cfg(feature = "gui")]
use eframe::{
    egui,
    egui::{Color32, Stroke, Vec2},
};
#[cfg(feature = "gui")]
use std::collections::HashSet;
#[cfg(feature = "gui")]
use csv::Writer;
#[cfg(feature = "gui")]
use std::fs::File;
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[cfg(feature = "gui")]
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
    prev_base_color: Color32,
}
#[cfg(feature = "gui")]
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
            prev_base_color: Color32::from_rgb(120, 120, 180),
        }
    }
}
#[cfg(feature = "gui")]
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
    focus_on: usize,
    request_formula_focus: bool,
}
#[cfg(feature = "gui")]
impl SpreadsheetApp {
    pub fn new(rows: usize, cols: usize, start_row: usize, start_col: usize) -> Self {
        let sheet = vec![
            vec![
                Cell {
                    value: Valtype::Int(0),
                    data: CellData::Empty,
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
            focus_on: 0,
            request_formula_focus: false,
        }
    }

    // Helper: Extract formula from cell
    fn get_cell_formula(&self, row: usize, col: usize) -> String {
        let cell = &self.sheet[row][col];
        match &cell.data {
            CellData::Empty => String::new(),

            CellData::Const => {
                if let Valtype::Int(val) = cell.value {
                    val.to_string()
                } else {
                    String::new()
                }
            }

            CellData::Ref { cell1 } => cell1.as_str().to_string(),

            CellData::CoC { op_code, value2 } => {
                if let Valtype::Int(val1) = &cell.value {
                    if let Valtype::Int(val2) = value2 {
                        format!("{}{}{}", val1, op_code, val2)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            }

            CellData::CoR {
                op_code,
                value2,
                cell2,
            } => {
                if let Valtype::Int(val1) = value2 {
                    format!("{}{}{}", val1, op_code, cell2.as_str())
                } else {
                    String::new()
                }
            }

            CellData::RoC {
                op_code,
                value2,
                cell1,
            } => {
                if let Valtype::Int(val2) = value2 {
                    format!("{}{}{}", cell1.as_str(), op_code, val2)
                } else {
                    String::new()
                }
            }

            CellData::RoR {
                op_code,
                cell1,
                cell2,
            } => {
                format!("{}{}{}", cell1, op_code, cell2)
            }

            CellData::Range {
                cell1,
                cell2,
                value2,
            } => {
                if let Valtype::Str(func) = value2 {
                    format!("{}({}:{})", func.as_str(), cell1.as_str(), cell2.as_str())
                } else {
                    String::new()
                }
            }

            CellData::SleepC => {
                if let Valtype::Int(val) = cell.value {
                    format!("SLEEP({})", val)
                } else {
                    String::new()
                }
            }

            CellData::SleepR { cell1 } => {
                format!("SLEEP({})", cell1)
            }

            CellData::Invalid => String::new(),
        }
    }

    // Update the value of the currently selected cell
    fn update_selected_cell(&mut self) {
        let total_rows = self.sheet.len();
        let total_cols = self.sheet[0].len();
        if let Some((r, c)) = self.selected {
            let old_cell = self.sheet[r][c].my_clone();
            parser::detect_formula(&mut self.sheet[r][c], &self.formula_input);
            parser::update_and_recalc(&mut self.sheet, total_rows, total_cols, r, c, old_cell);
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
                    let hint = if self.selected.is_some() {
                        "Enter formula or value..."
                    } else {
                        "Enter command..."
                    };
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.formula_input)
                            .id_salt("command bar")
                            .hint_text(hint)
                            .desired_width(ui.available_width() - 120.0)
                            .font(egui::TextStyle::Monospace)
                            .text_color(self.style.header_text),
                    );
                    if self.request_formula_focus {
                        response.request_focus();
                        self.request_formula_focus = false;
                    }
                    if response.gained_focus() {
                        self.focus_on = 2;
                    }
                    let process_formula = ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("Update Cell")
                                    .size(self.style.font_size)
                                    .color(self.style.selected_cell_text),
                            )
                            .fill(self.style.selected_cell_bg)
                            .min_size(egui::Vec2::new(100.0, 25.0)),
                        )
                        .clicked()
                        || ((self.focus_on == 2) && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                    if process_formula {
                        if self.selected.is_some() {
                            self.update_selected_cell();
                            self.editing_cell = false;
                        } else {
                            let cmd = self.formula_input.trim().to_lowercase();
                            self.process_command(&cmd);
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

    fn process_command(&mut self, cmd: &str) {
        match cmd {
            "q" => std::process::exit(0),
            "tr" => self.reset_theme(),
            "copy" => self.status_message = "Copy function not implemented yet".to_string(),
            "paste" => self.status_message = "Paste function not implemented yet".to_string(),
            "help" => self.show_command_help(),
            _ => {
                if cmd.starts_with("scroll_to ") {
                    if let Some(cell_ref) = cmd.strip_prefix("scroll_to ") {
                        self.scroll_to_cell = cell_ref.to_string();
                        self.process_scroll_to_cell();
                    }
                } else if cmd.starts_with("goto ") {
                    if let Some(cell_ref) = cmd.strip_prefix("goto ") {
                        self.goto_cell(cell_ref);
                    }
                } else if cmd.starts_with("w") {
                    let arg = &cmd[1..].trim();
                    if arg.is_empty() {
                        self.move_selection_n(Direction::Up, 1);
                    } else if let Ok(count) = arg.parse::<usize>() {
                        self.move_selection_n(Direction::Up, count);
                    }else {
                        self.status_message = format!("Unknown command: {}", cmd);
                    }
                }else if cmd.starts_with("csv ") {
                    let filename = cmd.strip_prefix("csv ").unwrap().trim();
                    self.export_to_csv(filename);
                } else if cmd.starts_with("s") {
                    let arg = &cmd[1..].trim();
                    if arg.is_empty() {
                        self.move_selection_n(Direction::Down, 1);
                    } else if let Ok(count) = arg.parse::<usize>() {
                        self.move_selection_n(Direction::Down, count);
                    }else {
                        self.status_message = format!("Unknown command: {}", cmd);
                    }
                } else if cmd.starts_with("a") {
                    let arg = &cmd[1..].trim();
                    if arg.is_empty() {
                        self.move_selection_n(Direction::Left, 1);
                    } else if let Ok(count) = arg.parse::<usize>() {
                        self.move_selection_n(Direction::Left, count);
                    }else {
                        self.status_message = format!("Unknown command: {}", cmd);
                    }
                } else if cmd.starts_with("d") {
                    let arg = &cmd[1..].trim();
                    if arg.is_empty() {
                        self.move_selection_n(Direction::Right, 1);
                    } else if let Ok(count) = arg.parse::<usize>() {
                        self.move_selection_n(Direction::Right, count);
                    }else {
                        self.status_message = format!("Unknown command: {}", cmd);
                    }
                } else {
                    self.status_message = format!("Unknown command: {}", cmd);
                }
            }
        }
    }
    fn export_to_csv(&mut self, filename: &str) {
        let filename = if filename.ends_with(".csv") {
            filename.to_string()
        } else {
            format!("{}.csv", filename)
        };

        match File::create(&filename) {
            Ok(file) => {
                let mut wtr = Writer::from_writer(file);
                for row in &self.sheet {
                    let record: Vec<String> = row.iter().map(|cell| match &cell.value {
                        Valtype::Int(n) => n.to_string(),
                        Valtype::Str(s) => s.to_string(),
                    }).collect();
                    
                    if let Err(e) = wtr.write_record(&record) {
                        self.status_message = format!("CSV write error: {}", e);
                        return;
                    }
                }
                wtr.flush().unwrap();
                self.status_message = format!("Exported to {}", filename);
            }
            Err(e) => self.status_message = format!("File error: {}", e),
        }
    }
    fn move_selection_n(&mut self, direction: Direction, amount: usize) {
        let total_rows = self.sheet.len();
        let total_cols = self.sheet[0].len();
        match direction {
            Direction::Up => crate::utils_gui::w(&mut self.start_row, amount),
            Direction::Down => crate::utils_gui::s(&mut self.start_row, total_rows, amount),
            Direction::Right => crate::utils_gui::d(&mut self.start_col, total_cols, amount),
            Direction::Left => crate::utils_gui::a(&mut self.start_row, amount),
        };
        self.status_message = format!(
            "Moved to cell {}{}",
            col_label(self.start_col),
            (self.start_row + 1).to_string()
        );
    }

    fn reset_theme(&mut self) {
        self.style = SpreadsheetStyle::default();
        self.status_message = "Theme reset to default".to_string();
    }

    fn goto_cell(&mut self, cell_ref: &str) {
        if let Some(pos) = cell_ref.chars().position(|c| c.is_ascii_digit()) {
            let col_str = &cell_ref[..pos];
            let row_str = &cell_ref[pos..];
            let mut col_index: usize = 0;
            for c in col_str.chars() {
                let c = c.to_ascii_uppercase();
                col_index = col_index * 26 + ((c as u8 - b'A') as usize + 1);
            }
            let col = col_index - 1;
            if let Ok(row) = row_str.parse::<usize>() {
                let row_index = row - 1;
                let total_rows = self.sheet.len();
                let total_cols = self.sheet[0].len();
                if row > 0 && row <= total_rows && col < total_cols {
                    self.selected = Some((row_index, col));
                    self.status_message = format!("Moved to cell {}", cell_ref);
                    return;
                }
            }
        }
        self.status_message = format!("Invalid cell reference: {}", cell_ref);
    }

    fn show_command_help(&mut self) {
        self.status_message =
            "Available commands: w,a,s,d (navigation), q (quit), theme_reset, help, goto [cell]"
                .to_string();
    }

    // Render the scroll-to-cell feature
    fn render_scroll_to_cell(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("Scroll to:")
                .size(self.style.font_size)
                .color(self.style.header_text),
        );
        let text_response = ui.add(
            egui::TextEdit::singleline(&mut self.scroll_to_cell)
                .hint_text("e.g. AB78")
                .desired_width(80.0)
                .font(egui::TextStyle::Monospace)
                .text_color(self.style.header_text),
        );
        if text_response.gained_focus() {
            self.focus_on = 1;
        }
        let enter_pressed = (self.focus_on == 1) && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let button_clicked = ui
            .add(
                egui::Button::new(
                    egui::RichText::new("Go")
                        .size(self.style.font_size)
                        .color(self.style.selected_cell_text),
                )
                .fill(self.style.selected_cell_bg)
                .min_size(egui::Vec2::new(60.0, 25.0)),
            )
            .clicked();
        if enter_pressed || button_clicked {
            self.process_scroll_to_cell();
        }
    }

    // Render the colour picker feature
    fn render_colour(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("Theme:")
                .size(self.style.font_size)
                .color(self.style.header_text),
        );
        let mut base_color = self.style.prev_base_color.clone();
        if ui.color_edit_button_srgba(&mut base_color).changed() {
            fn adjust_brightness(color: Color32, factor: f32) -> Color32 {
                let r = (color.r() as f32 * factor).min(255.0).max(0.0) as u8;
                let g = (color.g() as f32 * factor).min(255.0).max(0.0) as u8;
                let b = (color.b() as f32 * factor).min(255.0).max(0.0) as u8;
                Color32::from_rgb(r, g, b)
            }
            fn contrast_color(bg: Color32) -> Color32 {
                let r = bg.r() as f32;
                let g = bg.g() as f32;
                let b = bg.b() as f32;
                let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                if luminance < 128.0 {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(0, 0, 0)
                }
            }
            fn invert(bg: Color32) -> Color32 {
                let r = (255.0 - (bg.r() as f32)) as u8;
                let g = (255.0 - (bg.g() as f32)) as u8;
                let b = (255.0 - (bg.b() as f32)) as u8;
                Color32::from_rgb(r, g, b)
            }
            self.style.selected_cell_bg = invert(base_color);
            self.style.cell_bg_even = adjust_brightness(base_color, 0.8);
            self.style.cell_bg_odd = adjust_brightness(base_color, 1.2);
            self.style.cell_text = contrast_color(base_color);
            self.style.selected_cell_text = contrast_color(invert(base_color));
            self.style.grid_line = Stroke::new(1.0, adjust_brightness(base_color, 0.7));
            self.style.prev_base_color = base_color;
        }
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
                Valtype::Str(s) => s.as_str().to_string(),
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
        let rect =
            egui::Rect::from_min_size(rect.min, egui::Vec2::new(rect.width(), rect.height()));
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.formula_input)
                    .hint_text("Edit...")
                    .text_color(self.style.selected_cell_text)
                    .background_color(self.style.selected_cell_bg)
                    .vertical_align(egui::Align::Center)
                    .margin(egui::Vec2::new(3.0, 5.0)),
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
        let total_cols = self.sheet[0].len().min(self.start_col + 200);
        let total_rows = self.sheet.len().min(self.start_row + 100);
        let virtual_width = row_label_width + (total_cols - self.start_col) as f32 * cell_size.x;
        let virtual_height = header_height + (total_rows - self.start_row) as f32 * cell_size.y;
        let virtual_size = egui::vec2(virtual_width, virtual_height);
        let mut scroll_area = egui::ScrollArea::both()
            .id_salt((self.start_row, self.start_col))
            .drag_to_scroll(true)
            .auto_shrink([false, false]);
        if self.should_reset_scroll {
            scroll_area = scroll_area.scroll_offset(egui::Vec2::ZERO);
        }
        let mut scroll_offset = egui::Vec2::ZERO;
        scroll_area.show(ui, |ui| {
            let (virtual_rect, _) = ui.allocate_exact_size(virtual_size, egui::Sense::hover());
            scroll_offset = ui.clip_rect().min - virtual_rect.min;
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
            egui::Order::Background,
            egui::Id::new("pinned_headers"),
        ));
        let base_x = ui.min_rect().min.x;
        let base_y = ui.min_rect().min.y;
        // --- Column Headers (pinned vertically, scrolled horizontally) ---
        for col_idx in self.start_col..total_cols {
            let header_x = base_x - scroll_offset.x
                + (col_idx - self.start_col) as f32 * cell_size.x
                + row_label_width;
            let header_rect = egui::Rect::from_min_size(
                egui::pos2(header_x.max(base_x), base_y),
                egui::vec2(cell_size.x, header_height),
            );
            painter.rect_filled(header_rect, 0.0, self.style.header_bg);
            painter.text(
                header_rect.center(),
                egui::Align2::CENTER_CENTER,
                col_label(col_idx),
                egui::FontId::monospace(self.style.font_size),
                self.style.header_text,
            );
            use egui::epaint::StrokeKind;
            painter.rect_stroke(header_rect, 0.0, self.style.grid_line, StrokeKind::Middle);
        }
        // --- Row Labels (pinned horizontally, scrolled vertically) ---
        for row_idx in self.start_row..total_rows {
            let header_y = base_y - scroll_offset.y
                + (row_idx - self.start_row) as f32 * cell_size.y
                + header_height;
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

    // Handle keyboard events with dynamic viewport sizes.
    // The dynamic visible rows and columns are computed in update() and passed here.
    fn handle_keyboard_events(
        &mut self,
        ctx: &egui::Context,
        visible_rows: usize,
        visible_cols: usize,
    ) {
        ctx.input(|input| {
            if input.key_pressed(egui::Key::ArrowDown) {
                if let Some((row, col)) = self.selected {
                    if row + 1 < self.sheet.len() {
                        self.selected = Some((row + 1, col));
                        if row + 1 >= self.start_row + visible_rows {
                            self.start_row = row + 1 - visible_rows + 1;
                            self.should_reset_scroll = true;
                        }
                    }
                }
            } else if input.key_pressed(egui::Key::ArrowUp) {
                if let Some((row, col)) = self.selected {
                    if row > 0 {
                        self.selected = Some((row - 1, col));
                        if row - 1 < self.start_row {
                            self.start_row = row - 1;
                            self.should_reset_scroll = true;
                        }
                    }
                }
            } else if input.key_pressed(egui::Key::ArrowRight) {
                if let Some((row, col)) = self.selected {
                    if col + 1 < self.sheet[0].len() {
                        self.selected = Some((row, col + 1));
                        if col + 1 >= self.start_col + visible_cols {
                            self.start_col = col + 1 - visible_cols + 1;
                            self.should_reset_scroll = true;
                        }
                    }
                }
            } else if input.key_pressed(egui::Key::ArrowLeft) {
                if let Some((row, col)) = self.selected {
                    if col > 0 {
                        self.selected = Some((row, col - 1));
                        if col - 1 < self.start_col {
                            self.start_col = col - 1;
                            self.should_reset_scroll = true;
                        }
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
            self.selected = Some((i, j));
            self.formula_input = self.get_cell_formula(i, j);
            self.status_message = format!("Selected cell {}{}", col_label(j), i + 1);
        }
    }
}

#[cfg(feature = "gui")]
impl eframe::App for SpreadsheetApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals::dark());
        let mut new_selection = None;

        egui::TopBottomPanel::top("formula_panel").show(ctx, |ui| {
            self.render_formula_bar(ui);
            ui.horizontal(|ui| {
                self.render_scroll_to_cell(ui);
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(16.0);
                self.render_colour(ui);
                ui.add_space(16.0);
                ui.separator();
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(selection) = self.render_spreadsheet_grid(ui) {
                new_selection = Some(selection);
            }
            self.render_selected_cell_info(ui);
        });

        self.handle_selection_change(new_selection);

        let avail_rect = ctx.available_rect();
        let avail_size = avail_rect.size();
        let row_label_width = 30.0;
        let visible_rows = 33;
        let visible_cols =
            (((avail_size.x - row_label_width) / self.style.cell_size.x).ceil() as usize).max(1);

        self.handle_keyboard_events(ctx, visible_rows, visible_cols);
    }
}
