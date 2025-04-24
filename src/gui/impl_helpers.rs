use std::fs::File;

use csv::Writer;

use crate::gui::gui_defs::UndoAction;
use crate::{
    gui::gui_defs::SpreadsheetApp, gui::utils_gui::cell_data_to_formula_string,
    gui::utils_gui::col_label, gui::utils_gui::valtype_to_string, parser, Cell, CellData, HashSet,
    Valtype, STATUS, STATUS_CODE,
};

impl SpreadsheetApp {
    // Helper: Extract formula from cell
    pub fn get_cell_formula(&self, row: usize, col: usize) -> String {
        let key = (row * self.total_cols + col) as u32;
        if let Some(cell) = self.sheet.get(&key) {
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
        } else {
            String::new()
        }
    }

    // Update the value of the currently selected cell
    pub fn update_selected_cell(&mut self) {
        let total_rows = self.total_rows;
        let total_cols = self.total_cols;
        if let Some((r, c)) = self.selected {
            // Save the current state for undo before making changes
            self.push_undo_action(r, c);
            let idx = (r as u32) * (total_cols as u32) + (c as u32);
            let old_cell = self.sheet.get(&idx).cloned().unwrap_or(Cell {
                value: Valtype::Int(0),
                data: CellData::Empty,
                dependents: HashSet::new(),
            });
            let mut new_cell = old_cell.clone();
            parser::detect_formula(&mut new_cell, &self.formula_input);
            self.sheet.insert(idx, new_cell);
            parser::update_and_recalc(
                &mut self.sheet,
                &mut self.ranged,
                &mut self.is_range,
                (total_rows, total_cols),
                r,
                c,
                old_cell,
            );
            self.status_message = match unsafe { STATUS_CODE } {
                0 => format!("Updated cell {}{}", col_label(c), r + 1),
                code => STATUS[code].to_string()
            };
            unsafe {
                STATUS_CODE = 0;
            }
        }
    }

    pub fn export_to_csv(&mut self, filename: &str) {
        let filename = if filename.ends_with(".csv") {
            filename.to_string()
        } else {
            format!("{}.csv", filename)
        };

        match File::create(&filename) {
            Ok(file) => {
                let mut wtr = Writer::from_writer(file);
                for row in 0..self.total_rows {
                    let mut record: Vec<String> = Vec::with_capacity(self.total_cols);
                    for col in 0..self.total_cols {
                        let key = (row * self.total_cols + col) as u32;
                        if let Some(cell) = self.sheet.get(&key) {
                            let cell_str = match &cell.value {
                                Valtype::Int(n) => n.to_string(),
                                Valtype::Str(s) => s.to_string(),
                            };
                            record.push(cell_str);
                        } else {
                            record.push("0".to_string());
                        }
                    }

                    if let Err(e) = wtr.write_record(&record) {
                        self.status_message = format!("CSV write error: {}", e);
                        return;
                    }
                }

                if let Err(e) = wtr.flush() {
                    self.status_message = format!("CSV flush error: {}", e);
                    return;
                }

                self.status_message = format!("Exported to {}", filename);
            }
            Err(e) => self.status_message = format!("File error: {}", e),
        }
    }

    pub fn export_formulas_to_csv(&mut self, filename: &str) {
        let filename = if filename.ends_with(".csv") {
            filename.to_string()
        } else {
            format!("{}.csv", filename)
        };
        match File::create(&filename) {
            Ok(file) => {
                let mut wtr = Writer::from_writer(file);
                for row in 0..self.total_rows {
                    let mut record: Vec<String> = Vec::with_capacity(self.total_cols);
                    for col in 0..self.total_cols {
                        let key = (row * self.total_cols + col) as u32;
                        if let Some(cell) = self.sheet.get(&key) {
                            let formula_str = cell_data_to_formula_string(&cell.data)
                                .unwrap_or_else(|| valtype_to_string(&cell.value));
                            record.push(formula_str);
                        } else {
                            record.push("0".to_string());
                        }
                    }

                    if let Err(e) = wtr.write_record(&record) {
                        self.status_message = format!("CSV write error: {}", e);
                        return;
                    }
                }

                if let Err(e) = wtr.flush() {
                    self.status_message = format!("CSV flush error: {}", e);
                } else {
                    self.status_message = format!("Exported formulas to {}", filename);
                }
            }
            Err(e) => {
                self.status_message = format!("File error: {}", e);
            }
        }
    }

    // Handle cell selection changes
    pub fn handle_selection_change(&mut self, new_selection: Option<(usize, usize)>) {
        if let Some((i, j)) = new_selection {
            self.selected = Some((i, j));
            self.formula_input = self.get_cell_formula(i, j);
            self.status_message = format!("Selected cell {}{}", col_label(j), i + 1);
        }
    }

    pub fn goto_cell(&mut self, cell_ref: &str) {
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
                let total_rows = self.total_rows;
                let total_cols = self.total_cols;
                if row > 0 && row <= total_rows && col < total_cols {
                    self.selected = Some((row_index, col));
                    self.status_message = format!("Moved to cell {}", cell_ref);
                    return;
                }
            }
        }
        self.status_message = format!("Invalid cell reference: {}", cell_ref);
    }
}

impl SpreadsheetApp {
    pub fn copy_selected_cell(&mut self) {
        if let Some((row, col)) = self.selected {
            let key = (row * self.total_cols + col) as u32;

            if let Some(cell) = self.sheet.get(&key) {
                self.clipboard = Some(cell.clone());
                self.clipboard_formula = self.get_cell_formula(row, col);
                self.status_message = format!("Copied cell {}{}", col_label(col), row + 1);
            } else {
                let empty_cell = Cell {
                    value: Valtype::Int(0),
                    data: CellData::Empty,
                    dependents: HashSet::new(),
                };
                self.clipboard = Some(empty_cell);
                self.clipboard_formula = String::new();
                self.status_message = format!("Copied empty cell {}{}", col_label(col), row + 1);
            }
        } else {
            self.status_message = "No cell selected for copy".to_string();
        }
    }

    pub fn cut_selected_cell(&mut self) {
        self.copy_selected_cell();
        if let Some((row, col)) = self.selected {
            let key = (row * self.total_cols + col) as u32;
            if self.sheet.contains_key(&key) {
                let empty_cell = Cell {
                    value: Valtype::Int(0),
                    data: CellData::Empty,
                    dependents: HashSet::new(),
                };
                self.sheet.insert(key, empty_cell);
                self.status_message = format!("Moved cell {}{}", col_label(col), row + 1);
            } else {
                self.status_message = format!("No data to cut at {}{}", col_label(col), row + 1);
            }
        } else {
            self.status_message = "No cell selected for cut".to_string();
        }
    }

    // Push an action to the undo stack
    fn push_undo_action(&mut self, row: usize, col: usize) {
        let key = (row * self.total_cols + col) as u32;

        let old_cell = match self.sheet.get(&key) {
            Some(cell) => cell.clone(),
            None => Cell {
                value: Valtype::Int(0),
                data: CellData::Empty,
                dependents: HashSet::new(),
            },
        };

        let old_formula = self.get_cell_formula(row, col);
        self.undo_stack.push(UndoAction {
            position: (row, col),
            old_cell,
            old_formula,
        });
        self.redo_stack.clear();

        if self.undo_stack.len() > self.max_undo_levels {
            self.undo_stack.remove(0);
        }
    }

    // Undo the last action for the undo function
    pub fn undo(&mut self) {
        if let Some(action) = self.undo_stack.pop() {
            let (row, col) = action.position;
            let idx = (row as u32) * (self.total_cols as u32) + (col as u32);
            // Save current state for redo
            let current_cell = self.sheet.get(&idx).cloned().unwrap_or(Cell {
                value: Valtype::Int(0),
                data: CellData::Empty,
                dependents: HashSet::new(),
            });
            let current_formula = self.get_cell_formula(row, col);

            self.redo_stack.push(UndoAction {
                position: (row, col),
                old_cell: current_cell.clone(), // Clone here
                old_formula: current_formula,
            });
            *self.sheet.get_mut(&idx).unwrap() = action.old_cell;
            // Restore previous state
            self.formula_input = action.old_formula;

            // Update selection
            self.selected = Some((row, col));

            // Recalculate dependencies
            let total_rows = self.total_rows;
            let total_cols = self.total_cols;

            parser::update_and_recalc(
                &mut self.sheet,
                &mut self.ranged,
                &mut self.is_range,
                (total_rows, total_cols),
                row,
                col,
                current_cell,
            );

            self.status_message = format!("Undid change to cell {}{}", col_label(col), row + 1);
        } else {
            self.status_message = "Nothing to undo".to_string();
        }
    }

    pub fn paste_to_selected_cell(&mut self) {
        if let Some((row, col)) = self.selected {
            // Create local copies of any data needed from immutable borrows
            let clipboard_data = self.clipboard.as_ref().map(|cell| cell.clone());
            let clipboard_formula_copy = self.clipboard_formula.clone();

            // Now proceed with mutable operations
            if let Some(copied_cell) = clipboard_data {
                // Safe to mutably borrow self now
                self.push_undo_action(row, col);

                if !clipboard_formula_copy.is_empty() {
                    self.formula_input = clipboard_formula_copy;
                    self.update_selected_cell();
                } else {
                    let total_rows = self.total_rows;
                    let total_cols = self.total_cols;
                    let idx = (row as u32) * (total_cols as u32) + (col as u32);
                    let old_cell = self.sheet.get(&idx).cloned().unwrap_or(Cell {
                        value: Valtype::Int(0),
                        data: CellData::Empty,
                        dependents: HashSet::new(),
                    });
                    *self.sheet.get_mut(&idx).unwrap() = copied_cell;
                    // Recalculate dependencies
                    parser::update_and_recalc(
                        &mut self.sheet,
                        &mut self.ranged,
                        &mut self.is_range,
                        (total_rows, total_cols),
                        row,
                        col,
                        old_cell,
                    );
                }

                self.status_message = format!("Pasted to cell {}{}", col_label(col), row + 1);
            } else {
                self.status_message = "Nothing to paste".to_string();
            }
        } else {
            self.status_message = "No cell selected for paste".to_string();
        }
    }

    // Redo the last undone action
    pub fn redo(&mut self) {
        if let Some(action) = self.redo_stack.pop() {
            let (row, col) = action.position;

            // Save current state for undo
            let idx = (row as u32) * (self.total_cols as u32) + (col as u32);
            let current_cell = self.sheet.get(&idx).cloned().unwrap_or(Cell {
                value: Valtype::Int(0),
                data: CellData::Empty,
                dependents: HashSet::new(),
            });
            let current_formula = self.get_cell_formula(row, col);

            self.undo_stack.push(UndoAction {
                position: (row, col),
                old_cell: current_cell.clone(), // Clone here
                old_formula: current_formula,
            });

            // Restore redo state
            *self.sheet.get_mut(&idx).unwrap() = action.old_cell;
            self.formula_input = action.old_formula;

            // Update selection
            self.selected = Some((row, col));

            // Recalculate dependencies
            let total_rows = self.total_rows;
            let total_cols = self.total_cols;
            parser::update_and_recalc(
                &mut self.sheet,
                &mut self.ranged,
                &mut self.is_range,
                (total_rows, total_cols),
                row,
                col,
                current_cell,
            );

            self.status_message = format!("Redid change to cell {}{}", col_label(col), row + 1);
        } else {
            self.status_message = "Nothing to redo".to_string();
        }
    }
}
