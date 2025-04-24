use egui::{Color32, Stroke};

use crate::{
    gui::gui_defs::{Direction, SpreadsheetApp, SpreadsheetStyle},
    gui::utils_gui::{col_label, parse_cell_name},
    utils::to_indices,
    Valtype,
};

impl SpreadsheetApp {
    /// Renders the formula input bar at the top of the UI.
    ///
    /// # Arguments
    /// * `ui` - The mutable reference to the egui UI context.
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
                            let cmd = self.formula_input.clone();
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

    /// Processes commands entered in the formula bar.
    ///
    /// # Arguments
    /// * `cmd` - The command string to process.
    fn process_command(&mut self, cmd: &str) {
        let mut flag = true;
        match cmd {
            "q" => std::process::exit(0),
            "tr" => self.reset_theme(),
            "undo" => self.undo(),
            "redo" => self.redo(),
            "help" => self.show_command_help(),
            "rainbow1" => {
                self.style.rainbow = 1;
            }
            "rainbow2" => {
                self.style.rainbow = 2;
            }
            "matrix1" => {
                self.style.rainbow = 3;
            }
            "matrix2" => {
                self.style.rainbow = 5;
            }
            "matrix3" => {
                self.style.rainbow = 6;
            }
            "love" => {
                self.style.rainbow = 4;
            }
            _ => {
                if cmd.starts_with("copy ") {
                    if let Some(cell_ref) = cmd.strip_prefix("copy ") {
                        self.goto_cell(cell_ref);
                        self.copy_selected_cell();
                    }
                } else if cmd.starts_with("cut ") {
                    if let Some(cell_ref) = cmd.strip_prefix("cut ") {
                        self.goto_cell(cell_ref);
                        self.cut_selected_cell();
                    }
                } else if cmd.starts_with("paste ") {
                    if let Some(cell_ref) = cmd.strip_prefix("paste ") {
                        self.goto_cell(cell_ref);
                        self.paste_to_selected_cell();
                    }
                } else if cmd.starts_with("scroll_to ") {
                    if let Some(cell_ref) = cmd.strip_prefix("scroll_to ") {
                        self.scroll_to_cell = cell_ref.to_string();
                        self.process_scroll_to_cell();
                    }
                } else if cmd.starts_with("goto ") {
                    if let Some(cell_ref) = cmd.strip_prefix("goto ") {
                        self.goto_cell(cell_ref);
                        flag = false;
                    }
                } else if let Some(stripper) = cmd.strip_prefix("frequency ") {
                    let arg = stripper.trim(); // Ooh yes, gently remove that prefix
                    if arg.is_empty() {
                        self.status_message = "Please enter frequency".to_string();
                    } else if let Ok(count) = arg.parse::<f32>() {
                        self.style.frequency = count * 0.2 / 10.0;
                    } else {
                        self.status_message = format!("Unknown command: {}", cmd);
                    }
                } else if let Some(stripper) = cmd.strip_prefix("w") {
                    let arg = &stripper.trim();
                    if arg.is_empty() {
                        self.move_selection_n(Direction::Up, 1);
                    } else if let Ok(count) = arg.parse::<usize>() {
                        self.move_selection_n(Direction::Up, count);
                    } else {
                        self.status_message = format!("Unknown command: {}", cmd);
                    }
                } else if cmd.starts_with("csv ") {
                    let filename = cmd.strip_prefix("csv ").unwrap().trim();
                    self.export_to_csv(filename);
                } else if cmd.starts_with("fcsv ") {
                    let filename = cmd.strip_prefix("fcsv ").unwrap().trim();
                    self.export_formulas_to_csv(filename);
                } else if let Some(stripper) = cmd.strip_prefix("s") {
                    let arg = &stripper.trim();
                    if arg.is_empty() {
                        self.move_selection_n(Direction::Down, 1);
                    } else if let Ok(count) = arg.parse::<usize>() {
                        self.move_selection_n(Direction::Down, count);
                    } else {
                        self.status_message = format!("Unknown command: {}", cmd);
                    }
                } else if let Some(stripper) = cmd.strip_prefix("a") {
                    let arg = &stripper.trim();
                    if arg.is_empty() {
                        self.move_selection_n(Direction::Left, 1);
                    } else if let Ok(count) = arg.parse::<usize>() {
                        self.move_selection_n(Direction::Left, count);
                    } else {
                        self.status_message = format!("Unknown command: {}", cmd);
                    }
                } else if let Some(stripper) = cmd.strip_prefix("d") {
                    let arg = &stripper.trim();
                    if arg.is_empty() {
                        self.move_selection_n(Direction::Right, 1);
                    } else if let Ok(count) = arg.parse::<usize>() {
                        self.move_selection_n(Direction::Right, count);
                    } else {
                        self.status_message = format!("Unknown command: {}", cmd);
                    }
                } else if cmd.contains('=') {
                    let parts: Vec<&str> = cmd.splitn(2, '=').map(str::trim).collect();
                    if parts.len() == 2 {
                        let (cell_ref, formula) = (parts[0], parts[1]);
                        let (row, col) = to_indices(cell_ref);
                        self.selected = Some((row, col));
                        self.formula_input = formula.to_string();
                        self.update_selected_cell();
                        self.formula_input.clear();
                        self.selected = None;
                        self.request_formula_focus = true;
                    } else {
                        self.status_message = format!("unrecognized command: {}", cmd);
                    }
                } else {
                    self.status_message = format!("Unknown command: {}", cmd);
                }
            }
        }
        if flag {
            self.request_formula_focus = true;
        }
    }

    /// Resets the theme to its default settings.
    fn reset_theme(&mut self) {
        self.style = SpreadsheetStyle::default();
        self.status_message = "Theme reset to default".to_string();
    }

    /// Displays a help message with available commands.
    fn show_command_help(&mut self) {
        self.status_message = "Available commands: w,a,s,d Option<Amount> (navigation), q (quit), tr (theme_reset), help, goto [cell], scroll_to [cell], undo, redo, copy [cell], cut[cell], paste [cell], csv <filename>, fcsv <filename>, cell=formula,themes..".to_string();
    }

    /// Renders the "Scroll to" input field and button.
    ///
    /// # Arguments
    /// * `ui` - The mutable reference to the egui UI context.
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

    /// Renders the color picker and theme settings UI.
    ///
    /// # Arguments
    /// * `ui` - The mutable reference to the egui UI context.
    fn render_colour(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("Theme:")
                .size(self.style.font_size)
                .color(self.style.header_text),
        );
    
        // Original color picking logic when no rainbow mode is active
        let mut base_color = self.style.prev_base_color;
        if ui.color_edit_button_srgba(&mut base_color).changed() {
            self.style.get_cell_bg = None;
            self.style.rainbow = 0;
            fn adjust_brightness(color: Color32, factor: f32) -> Color32 {
                let r = (color.r() as f32 * factor).clamp(0.0, 255.0) as u8;
                let g = (color.g() as f32 * factor).clamp(0.0, 255.0) as u8;
                let b = (color.b() as f32 * factor).clamp(0.0, 255.0) as u8;
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
            // Semi-transparent range selection background with adjusted brightness
            self.style.range_selection_bg = Color32::from_rgba_unmultiplied(
                invert(base_color).r(),
                invert(base_color).g(),
                (base_color).b(),
                180, // 70% opacity
            );
            self.style.range_selection_text = contrast_color(invert(base_color));
        }
        // Check if Matrix Rain effect is active (matrix3)
        else if self.style.rainbow == 6 {
            let time = ui.ctx().input(|i| i.time);
            let time_f32 = time as f32;
    
            let matrix_bright_green = Color32::from_rgb(0, 255, 70);
            let black = Color32::from_rgb(0, 0, 0);
    
            self.style.cell_bg_even = black;
            self.style.cell_bg_odd = black;
            self.style.cell_text = matrix_bright_green;
            self.style.selected_cell_bg = matrix_bright_green;
            self.style.selected_cell_text = black;
            self.style.grid_line = Stroke::new(1.0, Color32::from_rgb(0, 0, 0));
    
            use rand::Rng;
            if self.style.matrix_raindrops.is_empty() {
                let columns = 50;
                let mut rng = rand::thread_rng();
                for _i in 0..columns {
                    let column = rng.gen_range(0..100);
                    let row = rng.gen_range(0..100);
                    let speed = rng.gen_range(3.0..8.0);
                    let length = rng.gen_range(5..15);
                    self.style.matrix_raindrops.push((column, row, speed, length));
                }
            }
    
            for (_, row, speed, _) in &mut self.style.matrix_raindrops {
                *row = (*row + (time_f32 * *speed * self.style.frequency / 20.0) as usize) % 200;
            }
    
            let matrix_raindrops = self.style.matrix_raindrops.clone();
            let matrix_bright_green_copy = matrix_bright_green;
            self.style.get_cell_bg = Some(Box::new(move |row, col| {
                for &(drop_col, drop_row, _, length) in &matrix_raindrops {
                    if col % 100 == drop_col {
                        let cell_pos = row % 200;
                        let head_pos = drop_row;
                        if cell_pos == head_pos {
                            return matrix_bright_green_copy;
                        }
                        if cell_pos < head_pos && head_pos - cell_pos <= length {
                            let fade = (head_pos - cell_pos) as f32 / length as f32;
                            let green_value = ((1.0 - fade) * 255.0) as u8;
                            return Color32::from_rgb(0, green_value, 0);
                        }
                    }
                }
                black
            }));
    
            // Semi-transparent green for range selection, pulsing with animation
            let pulse = ((time_f32 * 1.5 * self.style.frequency / 0.2).sin() * 0.3 + 1.0).clamp(0.7, 1.3);
            self.style.range_selection_bg = Color32::from_rgba_unmultiplied(
                0,
                (255.0 * pulse) as u8,
                0,
                150, // 60% opacity
            );
            self.style.range_selection_text = Color32::from_rgb(20, 20, 20); // Dark for contrast
    
            ui.horizontal(|ui| {
                ui.label("Matrix Speed:");
                ui.add(egui::Slider::new(&mut self.style.frequency, 0.05..=0.5).logarithmic(true));
            });
    
            ui.ctx().request_repaint();
            return;
        }
        // Matrix theme (matrix1)
        else if self.style.rainbow == 3 {
            self.style.get_cell_bg = None;
            let matrix_green = Color32::from_rgb(0, 255, 0);
            let black = Color32::from_rgb(0, 0, 0);
            let white = Color32::from_rgb(255, 255, 255);
    
            self.style.cell_bg_even = black;
            self.style.cell_bg_odd = black;
            self.style.cell_text = matrix_green;
            self.style.selected_cell_bg = matrix_green;
            self.style.selected_cell_text = white;
            self.style.grid_line = Stroke::new(1.0, Color32::from_rgb(0, 128, 0));
    
            let time = ui.ctx().input(|i| i.time);
            let time_f32 = time as f32;
            let pulse = ((time_f32 * 1.55 * self.style.frequency / 0.2).sin() * 0.3 + 1.0).clamp(0.7, 1.3);
    
            self.style.cell_text = Color32::from_rgb(0, (255.0 * pulse) as u8, 150);
            self.style.range_selection_bg = Color32::from_rgba_unmultiplied(
                0,
                (255.0 * pulse) as u8,
                0,
                150,
            );
            self.style.range_selection_text = Color32::from_rgb(20, 20, 20);
    
            ui.ctx().request_repaint();
            return;
        }
        // Matrix theme (matrix2)
        else if self.style.rainbow == 5 {
            self.style.get_cell_bg = None;
            let matrix_green = Color32::from_rgb(0, 255, 0);
            let black = Color32::from_rgb(0, 0, 0);
            let white = Color32::from_rgb(255, 255, 255);
    
            self.style.cell_bg_even = black;
            self.style.cell_bg_odd = black;
            self.style.cell_text = matrix_green;
            self.style.selected_cell_bg = matrix_green;
            self.style.selected_cell_text = white;
            self.style.grid_line = Stroke::new(1.0, Color32::from_rgb(0, 0, 0));
    
            let time = ui.ctx().input(|i| i.time);
            let time_f32 = time as f32;
            let pulse = ((time_f32 * 1.5 * self.style.frequency / 0.2).sin() * 0.3 + 1.0).clamp(0.7, 1.3);
    
            self.style.cell_text = Color32::from_rgb(0, (255.0 * pulse) as u8, 0);
            // Semi-transparent pulsing green for range selection
            self.style.range_selection_bg = Color32::from_rgba_unmultiplied(
                0,
                (255.0 * pulse) as u8,
                0,
                150, // 60% opacity
            );
            self.style.range_selection_text = Color32::from_rgb(20, 20, 20);
    
            ui.ctx().request_repaint();
            return;
        }
        // Love theme
        else if self.style.rainbow == 4 {
            self.style.get_cell_bg = None;
            let soft_pink = Color32::from_rgb(255, 192, 203);
            let deep_pink = Color32::from_rgb(193, 28, 132);
            let cream = Color32::from_rgb(255, 248, 231);
            let burgundy = Color32::from_rgb(128, 0, 32);
            let light_gold = Color32::from_rgb(250, 214, 165);
    
            self.style.cell_bg_even = soft_pink;
            self.style.cell_bg_odd = Color32::from_rgb(255, 218, 224);
            self.style.selected_cell_bg = deep_pink;
            self.style.selected_cell_text = cream;
            self.style.cell_text = burgundy;
            self.style.grid_line = Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(
                    light_gold.r(),
                    light_gold.g(),
                    light_gold.b(),
                    180,
                ),
            );
    
            let time = ui.ctx().input(|i| i.time);
            let time_f32 = time as f32;
            let beat = (time_f32 % 4.0) * 1.5;
            let pulse = if beat < 0.4 {
                0.5
            } else if beat < 0.7 {
                1.3
            } else if beat < 1.0 {
                0.6
            } else {
                1.3
            };
    
            self.style.selected_cell_bg = Color32::from_rgb(
                (deep_pink.r() as f32 * pulse) as u8,
                (deep_pink.g() as f32 * pulse) as u8,
                (deep_pink.b() as f32 * pulse) as u8,
            );
            // Semi-transparent pulsing pink for range selection
            self.style.range_selection_bg = Color32::from_rgba_unmultiplied(
                (soft_pink.r() as f32 * pulse) as u8,
                (soft_pink.g() as f32 * pulse) as u8,
                (soft_pink.b() as f32 * pulse) as u8,
                160, // 63% opacity
            );
            self.style.range_selection_text = cream;
    
            ui.ctx().request_repaint();
            return;
        }
        // Rainbow2 mode
        else if self.style.rainbow == 2 {
            let time = ui.ctx().input(|i| i.time);
            let time_f32 = time as f32;
            self.style.get_cell_bg = None;
            let frequency: f32 = self.style.frequency;
    
            fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
                let h = h % 360.0;
                let c = v * s;
                let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
                let m = v - c;
    
                let (r, g, b) = if h < 60.0 {
                    (c, x, 0.0)
                } else if h < 120.0 {
                    (x, c, 0.0)
                } else if h < 180.0 {
                    (0.0, c, x)
                } else if h < 240.0 {
                    (0.0, x, c)
                } else if h < 300.0 {
                    (x, 0.0, c)
                } else {
                    (c, 0.0, x)
                };
    
                let r = ((r + m) * 255.0) as u8;
                let g = ((g + m) * 255.0) as u8;
                let b = ((b + m) * 255.0) as u8;
    
                (r, g, b)
            }
    
            let hue = (time_f32 * frequency * 100.0) % 360.0;
            let (r, g, b) = hsv_to_rgb(hue, 0.9, 0.9);
            let base_color = Color32::from_rgb(r, g, b);
    
            fn adjust_brightness(color: Color32, factor: f32) -> Color32 {
                let r = (color.r() as f32 * factor).clamp(0.0, 255.0) as u8;
                let g = (color.g() as f32 * factor).clamp(0.0, 255.0) as u8;
                let b = (color.b() as f32 * factor).clamp(0.0, 255.0) as u8;
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
            // Semi-transparent cycling color for range selection
            self.style.range_selection_bg = Color32::from_rgba_unmultiplied(r, g, b, 160); // 63% opacity
            self.style.range_selection_text = contrast_color(base_color);
    
            ui.ctx().request_repaint();
        }
        // Rainbow1 mode
        else if self.style.rainbow == 1 {
            self.style.get_cell_bg = None;
            let time = ui.ctx().input(|i| i.time);
            let time_f32 = time as f32;
            let frequency: f32 = self.style.frequency;
    
            let red = ((std::f32::consts::PI * frequency * time_f32).sin() * 0.5 + 0.5) * 255.0;
            let green = ((std::f32::consts::PI * frequency * time_f32 + 2.0 * std::f32::consts::PI / 3.0)
                .sin() * 0.5 + 0.5) * 255.0;
            let blue = ((std::f32::consts::PI * frequency * time_f32 + 4.0 * std::f32::consts::PI / 3.0)
                .sin() * 0.5 + 0.5) * 255.0;
            let primary_color = Color32::from_rgb(red as u8, green as u8, blue as u8);
    
            let phase_shift: f32 = std::f32::consts::PI / 2.0;
            let red2 = ((std::f32::consts::PI * frequency * time_f32 + phase_shift).sin() * 0.5 + 0.5)
                * 255.0;
            let green2 = ((std::f32::consts::PI * frequency * time_f32
                + 2.0 * std::f32::consts::PI / 3.0
                + phase_shift)
                .sin() * 0.5
                + 0.5) * 255.0;
            let blue2 = ((std::f32::consts::PI * frequency * time_f32
                + 4.0 * std::f32::consts::PI / 3.0
                + phase_shift)
                .sin() * 0.5
                + 0.5) * 255.0;
            let secondary_color = Color32::from_rgb(red2 as u8, green2 as u8, blue2 as u8);
    
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
    
            self.style.cell_bg_even = primary_color;
            self.style.cell_bg_odd = secondary_color;
            self.style.selected_cell_bg = Color32::from_rgb(
                (255.0 - red) as u8,
                (255.0 - green) as u8,
                (255.0 - blue) as u8,
            );
            self.style.cell_text = contrast_color(primary_color);
            self.style.selected_cell_text = contrast_color(self.style.selected_cell_bg);
            self.style.grid_line = Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(
                    (blue * 0.7) as u8,
                    (red * 0.7) as u8,
                    (green * 0.7) as u8,
                    200,
                ),
            );
            // Semi-transparent blend of primary and secondary colors for range selection
            self.style.range_selection_bg = Color32::from_rgba_unmultiplied(
                ((red + red2) / 2.0) as u8,
                ((green + green2) / 2.0) as u8,
                ((blue + blue2) / 2.0) as u8,
                160, // 63% opacity
            );
            self.style.range_selection_text = contrast_color(primary_color);
    
            ui.ctx().request_repaint();
            return;
        }
    }
    /// Processes the "scroll to" action, updating the view to the specified cell.
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

    /// Renders the save dialog for exporting the spreadsheet to CSV.
    ///
    /// # Arguments
    /// * `ui` - The mutable reference to the egui UI context.
    fn render_save(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("Save as:")
                .size(self.style.font_size)
                .color(self.style.header_text),
        );

        // Add the filename input field
        let response = ui.add(
            egui::TextEdit::singleline(&mut self.save_filename)
                .hint_text("filename")
                .desired_width(200.0)
                .font(egui::TextStyle::Monospace)
                .text_color(self.style.header_text),
        );

        // Auto-focus the input field when dialog opens
        if self.show_save_dialog && self.focus_on == 0 {
            response.request_focus();
            self.focus_on = 3; // Use 3 as the focus ID for save dialog
        }

        // Handle Enter key and Save button
        let enter_pressed = (self.focus_on == 3) && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let save_clicked = ui
            .add(
                egui::Button::new(
                    egui::RichText::new("Save")
                        .size(self.style.font_size)
                        .color(self.style.selected_cell_text),
                )
                .fill(self.style.selected_cell_bg)
                .min_size(egui::Vec2::new(60.0, 25.0)),
            )
            .clicked();

        if (enter_pressed || save_clicked) && !self.save_filename.is_empty() {
            let filename = self.save_filename.clone();
            self.export_to_csv(&filename);
            self.show_save_dialog = false;
            self.focus_on = 0;
        }

        // Handle Cancel button and Escape key
        let cancel_clicked = ui
            .add(
                egui::Button::new(
                    egui::RichText::new("Cancel")
                        .size(self.style.font_size)
                        .color(self.style.header_text),
                )
                .min_size(egui::Vec2::new(60.0, 25.0)),
            )
            .clicked();

        if cancel_clicked || (self.focus_on == 3 && ui.input(|i| i.key_pressed(egui::Key::Escape)))
        {
            self.show_save_dialog = false;
            self.focus_on = 0;
            self.save_filename.clear();
        }
    }

    /// Renders a single cell in the spreadsheet grid.
    ///
    /// # Arguments
    /// * `ui` - The mutable reference to the egui UI context.
    /// * `row` - The row index of the cell.
    /// * `col` - The column index of the cell.
    /// * `rect` - The rectangular area to render the cell in.
    ///
    /// # Returns
    /// An optional tuple `(usize, usize)` representing the new selection if the cell was clicked.
    fn render_cell(
        &mut self,
        ui: &mut egui::Ui,
        row: usize,
        col: usize,
        rect: egui::Rect,
    ) -> Option<(usize, usize)> {
        let is_selected = self.selected == Some((row, col));
        let is_in_range = self.is_in_selected_range(row, col);
        let mut new_selection = None;
        if is_selected && self.editing_cell {
            self.render_editable_cell(ui, rect);
        } else {
            let key = (row * self.total_cols + col) as u32;
            let text = if let Some(cell) = self.sheet.get(&key) {
                match &cell.value {
                    Valtype::Int(n) => n.to_string(),
                    Valtype::Str(s) => s.as_str().to_string(),
                }
            } else {
                "0".to_string()
            };
    
            let bg_color = if is_selected {
                self.style.selected_cell_bg
            } else if is_in_range {
                self.style.range_selection_bg
            } else if let Some(get_bg) = &self.style.get_cell_bg {
                get_bg(row, col)
            } else if row % 2 == 0 {
                self.style.cell_bg_even
            } else {
                self.style.cell_bg_odd
            };
    
            let text_color = if is_selected {
                self.style.selected_cell_text
            } else if is_in_range {
                self.style.range_selection_text
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
    
            if response.clicked_by(egui::PointerButton::Primary) {
                self.is_selecting_range = false;
                self.range_end = None;
                self.range_start = None;
                new_selection = Some((row, col));
                if self.selected == Some((row, col)) {
                    self.editing_cell = true;
                } else {
                    self.selected = Some((row, col));
                }
            }
            if response.clicked_by(egui::PointerButton::Secondary) {
                if !self.is_selecting_range {
                    self.range_start = Some((row, col));
                    self.is_selecting_range = true;
                    self.status_message = format!("Range selection started at {}{}", col_label(col), row + 1);
                } else {
                    self.range_end = Some((row, col));
                    self.is_selecting_range = false;
                    if let (Some(start), Some(end)) = (self.range_start, self.range_end) {
                        let min_row = start.0.min(end.0);
                        let max_row = start.0.max(end.0);
                        let min_col = start.1.min(end.1);
                        let max_col = start.1.max(end.1);
                        self.status_message = format!(
                            "Selected range {}{}:{}{}",
                            col_label(min_col),
                            min_row + 1,
                            col_label(max_col),
                            max_row + 1
                        );
                    }
                }
            }
        }
        new_selection
    }
    /// Checks if a cell is within the currently selected range.
    ///
    /// # Arguments
    /// * `row` - The row index of the cell.
    /// * `col` - The column index of the cell.
    ///
    /// # Returns
    /// A boolean indicating whether the cell is within the selected range.
    fn is_in_selected_range(&self, row: usize, col: usize) -> bool {
        if let (Some(start), Some(end)) = (self.range_start, self.range_end) {
            let min_row = start.0.min(end.0);
            let max_row = start.0.max(end.0);
            let min_col = start.1.min(end.1);
            let max_col = start.1.max(end.1);
            row >= min_row && row <= max_row && col >= min_col && col <= max_col
        } else if self.is_selecting_range {
            self.range_start == Some((row, col))
        } else {
            false
        }
    }

    /// Renders an editable cell when editing is active.
    ///
    /// # Arguments
    /// * `ui` - The mutable reference to the egui UI context.
    /// * `rect` - The rectangular area to render the editable cell in.
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

    /// Renders the main spreadsheet grid with cells, headers, and row labels.
    ///
    /// # Arguments
    /// * `ui` - The mutable reference to the egui UI context.
    ///
    /// # Returns
    /// An optional tuple `(usize, usize)` representing the new selection if a cell was clicked.
    fn render_spreadsheet_grid(&mut self, ui: &mut egui::Ui) -> Option<(usize, usize)> {
        let mut new_selection = None;
        let cell_size = self.style.cell_size;
        let row_label_width = 30.0;
        let header_height = cell_size.y;
        let total_cols = self.total_cols.min(self.start_col + 300);
        let total_rows = self.total_rows.min(self.start_row + 500);
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

    /// Displays information about the currently selected cell.
    ///
    /// # Arguments
    /// * `ui` - The mutable reference to the egui UI context.
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

    /// Handles keyboard events for navigation and other actions.
    ///
    /// # Arguments
    /// * `ctx` - The egui context for input handling.
    /// * `visible_rows` - The number of visible rows in the viewport.
    /// * `visible_cols` - The number of visible columns in the viewport.
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
                self.formula_input.clear();
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
                self.formula_input.clear();
            } else if input.key_pressed(egui::Key::ArrowRight) {
                if let Some((row, col)) = self.selected {
                    if col + 1 < self.total_cols {
                        self.selected = Some((row, col + 1));
                        if col + 1 >= self.start_col + visible_cols {
                            self.start_col = col + 1 - visible_cols + 1;
                            self.should_reset_scroll = true;
                        }
                    }
                }
                self.formula_input.clear();
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
                self.formula_input.clear();
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
            } else if input.key_pressed(egui::Key::Space) {
                if let Some((row, col)) = self.selected {
                    self.formula_input = self.get_cell_formula(row, col);
                    self.editing_cell = true;
                    self.request_formula_focus = true;
                }
            }
            if input.modifiers.ctrl {
                if input.key_pressed(egui::Key::S) {
                    self.show_save_dialog = true;
                    self.focus_on = 0;
                } else if input.key_pressed(egui::Key::E) {
                    self.copy_selected_cell();
                } else if input.key_pressed(egui::Key::R) {
                    self.paste_to_selected_cell();
                } else if input.key_pressed(egui::Key::T) {
                    self.cut_selected_cell();
                } else if input.key_pressed(egui::Key::Z) {
                    self.undo();
                } else if input.key_pressed(egui::Key::Y)
                    || (input.modifiers.shift && input.key_pressed(egui::Key::Z))
                {
                    self.redo();
                }
            }
        });
    }
}

impl eframe::App for SpreadsheetApp {
    /// Updates the application state and renders the UI for each frame.
    ///
    /// # Arguments
    /// * `ctx` - The egui context for rendering and input handling.
    /// * `_frame` - A mutable reference to the eframe frame (unused).
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
                if self.show_save_dialog {
                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(16.0);
                    self.render_save(ui);
                }
                ui.add_space(8.0);
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
        let visible_rows = 31;
        let visible_cols =
            (((avail_size.x - row_label_width) / self.style.cell_size.x).ceil() as usize).max(1);

        self.handle_keyboard_events(ctx, visible_rows, visible_cols - 1);
    }
}
