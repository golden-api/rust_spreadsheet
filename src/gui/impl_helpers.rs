use std::fs::File;

use csv::Writer;
use std::error::Error;
use crate::gui::gui_defs::UndoAction;
use crate::{
    CellData,
    STATUS,
    STATUS_CODE,
    Valtype,
    gui::gui_defs::SpreadsheetApp,
    gui::utils_gui::cell_data_to_formula_string,
    gui::utils_gui::col_label,
    gui::utils_gui::valtype_to_string,
    gui::utils_gui::parse_cell_name,
    parser,Cell,HashSet
};
impl SpreadsheetApp {
    pub fn get_cell_formula(&self, row: usize, col: usize) -> String {
        if row >= self.total_rows || col >= self.total_cols {
            return String::new();
        }
        let index = (row as u32) * (self.total_cols as u32) + (col as u32);
        let cell = self.sheet.get(&index).cloned().unwrap_or_else(|| Cell {
            value: Valtype::Int(0),
            data: CellData::Empty,
            dependents: HashSet::new(),
        });
        match cell.data {
            CellData::Empty => String::new(),
            CellData::Const => match cell.value {
                Valtype::Int(n) => n.to_string(),
                Valtype::Str(s) => s.to_string(),
            },
            CellData::Ref { cell1 } => format!("={}", cell1),
            CellData::CoC { op_code, value2 } => {
                format!("={}{}{}", "", op_code, valtype_to_string(&value2))
            }
            CellData::CoR { op_code, value2, cell2 } => {
                format!("={}{}{}", valtype_to_string(&value2), op_code, cell2)
            }
            CellData::RoC { op_code, value2, cell1 } => {
                format!("={}{}{}", cell1, op_code, valtype_to_string(&value2))
            }
            CellData::RoR { op_code, cell1, cell2 } => {
                format!("={}{}{}", cell1, op_code, cell2)
            }
            CellData::Range { cell1, cell2, value2 } => match value2 {
                Valtype::Str(s) => format!("={}({}:{})", s, cell1, cell2),
                _ => format!("=RANGE({}:{},{})", cell1, cell2, valtype_to_string(&value2)),
            },
            CellData::SleepC => "=SLEEP()".to_string(),
            CellData::SleepR { cell1 } => format!("=SLEEP({})", cell1),
            CellData::Invalid => "#INVALID".to_string(),
        }
    }
    
    fn push_undo_action(&mut self, row: usize, col: usize) {
        if row >= self.total_rows || col >= self.total_cols {
            return;
        }
        let index = (row as u32) * (self.total_cols as u32) + (col as u32);
        let old_cell = self.sheet.get(&index).cloned().unwrap_or_else(|| Cell {
            value: Valtype::Int(0),
            data: CellData::Empty,
            dependents: HashSet::new(),
        });
        let old_formula = self.get_cell_formula(row, col);
        self.undo_stack.push(UndoAction {
            position: (row, col),
            old_cell,
            old_formula,
        });
        if self.undo_stack.len() > self.max_undo_levels {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }
    
    pub fn update_selected_cell(&mut self) {
        if let Some((r, c)) = self.selected {
            if r >= self.total_rows || c >= self.total_cols {
                self.status_message = "Selected cell out of bounds".to_string();
                return;
            }
            self.push_undo_action(r, c);
            let index = (r as u32) * (self.total_cols as u32) + (c as u32);
            let old_cell = self.sheet.get(&index).cloned().unwrap_or(Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() });
            let mut new_cell = old_cell.clone();
            parser::detect_formula(&mut new_cell, &self.formula_input);
            self.sheet.insert(index, new_cell);
            parser::update_and_recalc(
                &mut self.sheet,
                &mut self.ranged,
                &mut self.is_range,
                self.total_rows,
                self.total_cols,
                r,
                c,
                old_cell
            );
            self.status_message = "Cell updated".to_string();
        } else {
            self.status_message = "No cell selected".to_string();
        }
    }
    
    pub fn cut_selected_cell(&mut self) {
        if let Some((row, col)) = self.selected {
            if row >= self.total_rows || col >= self.total_cols {
                self.status_message = "Selected cell out of bounds".to_string();
                return;
            }
            self.copy_selected_cell();
            let index = (row as u32) * (self.total_cols as u32) + (col as u32);
            self.sheet.insert(index, Cell {
                value: Valtype::Int(0),
                data: CellData::Empty,
                dependents: HashSet::new(),
            });
            parser::update_and_recalc(
                &mut self.sheet,
                &mut self.ranged,
                &mut self.is_range,
                self.total_rows,
                self.total_cols,
                row,
                col,
                Cell {
                    value: Valtype::Int(0),
                    data: CellData::Empty,
                    dependents: HashSet::new(),
                },
            );
            self.status_message = "Cell cut".to_string();
        } else {
            self.status_message = "No cell selected".to_string();
        }
    }
    
    pub fn copy_selected_cell(&mut self) {
        if let Some((row, col)) = self.selected {
            if row >= self.total_rows || col >= self.total_cols {
                self.status_message = "Selected cell out of bounds".to_string();
                return;
            }
            let index = (row as u32) * (self.total_cols as u32) + (col as u32);
            self.clipboard = self.sheet.get(&index).cloned();
            self.clipboard_formula = self.get_cell_formula(row, col);
            self.status_message = "Cell copied".to_string();
        } else {
            self.status_message = "No cell selected".to_string();
        }
    }
    
    pub fn paste_to_selected_cell(&mut self) {
        if let Some((row, col)) = self.selected {
            if row >= self.total_rows || col >= self.total_cols {
                self.status_message = "Selected cell out of bounds".to_string();
                return;
            }
            if let Some(cell) = self.clipboard.clone() {
                self.push_undo_action(row, col);
                let idx = (row as u32) * (self.total_cols as u32) + (col as u32);
                let old_cell = self.sheet.get(&idx).cloned().unwrap_or(Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() });
                let mut new_cell = old_cell.clone();
                parser::detect_formula(&mut new_cell, &self.formula_input);
                self.sheet.insert(idx, new_cell);
                parser::update_and_recalc(
                    &mut self.sheet,
                    &mut self.ranged,
                    &mut self.is_range,
                    self.total_rows,
                    self.total_cols,
                    row,
                    col,
                    old_cell
                );
                self.status_message = "Cell pasted".to_string();
            } else {
                self.status_message = "No data in clipboard".to_string();
            }
        } else {
            self.status_message = "No cell selected".to_string();
        }
    }
    
    pub fn handle_selection_change(&mut self, new_selection: Option<(usize, usize)>) {
        self.selected = new_selection;
        if let Some((row, col)) = new_selection {
            if row >= self.total_rows || col >= self.total_cols {
                self.status_message = "Selected cell out of bounds".to_string();
                self.selected = None;
                return;
            }
            self.formula_input = self.get_cell_formula(row, col);
            self.status_message = format!("Selected cell {}{}", col_label(col), row + 1);
        } else {
            self.formula_input.clear();
            self.status_message = "No cell selected".to_string();
        }
    }
    
    pub fn goto_cell(&mut self, cell_ref: &str) {
        let result = parse_cell_name(cell_ref);
        match result {
            Some((row, col)) => {
                if row >= self.total_rows || col >= self.total_cols {
                    self.status_message = "Cell reference out of bounds".to_string();
                    return;
                }
                self.selected = Some((row, col));
                self.start_row = row.saturating_sub(10);
                self.start_col = col.saturating_sub(10);
                self.should_reset_scroll = true;
                self.formula_input = self.get_cell_formula(row, col);
                self.status_message = format!("Moved to cell {}{}", col_label(col), row + 1);
            }
            None => {
                self.status_message = "Invalid cell reference".to_string();
            }
        }
    }
    
    pub fn undo(&mut self) {
        if let Some(action) = self.undo_stack.pop() {
            let (row, col) = action.position;
            if row >= self.total_rows || col >= self.total_cols {
                self.status_message = "Undo position out of bounds".to_string();
                return;
            }
            let index = (row as u32) * (self.total_cols as u32) + (col as u32);
            let current_cell = self.sheet.get(&index).cloned().unwrap_or_else(|| Cell {
                value: Valtype::Int(0),
                data: CellData::Empty,
                dependents: HashSet::new(),
            });
            let current_formula = self.get_cell_formula(row, col);
            self.redo_stack.push(UndoAction {
                position: (row, col),
                old_cell: current_cell,
                old_formula: current_formula,
            });
            self.sheet.insert(index, action.old_cell);
            let old_cell = self.sheet.get(&index).cloned().unwrap_or(Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() });
            let mut new_cell = old_cell.clone();
            parser::detect_formula(&mut new_cell, &self.formula_input);
            parser::update_and_recalc(
                &mut self.sheet,
                &mut self.ranged,
                &mut self.is_range,
                self.total_rows,
                self.total_cols,
                row,
                col,
                old_cell
            );
        self.formula_input = action.old_formula;
            self.status_message = "Undo performed".to_string();
        } else {
            self.status_message = "Nothing to undo".to_string();
        }
    }
    
    pub fn redo(&mut self) {
        if let Some(action) = self.redo_stack.pop() {
            let (row, col) = action.position;
            if row >= self.total_rows || col >= self.total_cols {
                self.status_message = "Redo position out of bounds".to_string();
                return;
            }
            let index = (row as u32) * (self.total_cols as u32) + (col as u32);
            let current_cell = self.sheet.get(&index).cloned().unwrap_or_else(|| Cell {
                value: Valtype::Int(0),
                data: CellData::Empty,
                dependents: HashSet::new(),
            });
            let current_formula = self.get_cell_formula(row, col);
            self.undo_stack.push(UndoAction {
                position: (row, col),
                old_cell: current_cell,
                old_formula: current_formula,
            });
            self.sheet.insert(index, action.old_cell);
            let old_cell = self.sheet.get(&index).cloned().unwrap_or(Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() });
            let mut new_cell = old_cell.clone();
            parser::detect_formula(&mut new_cell, &self.formula_input);
            self.sheet.insert(index, new_cell);
            parser::update_and_recalc(
                &mut self.sheet,
                &mut self.ranged,
                &mut self.is_range,
                self.total_rows,
                self.total_cols,
                row,
                col,
                old_cell
            );
            self.formula_input = action.old_formula;
            self.status_message = "Redo performed".to_string();
        } else {
            self.status_message = "Nothing to redo".to_string();
        }
    }
    pub fn export_to_csv(&self, filename: &str) -> Result<(), Box<dyn Error>> {
        let mut writer = Writer::from_path(filename)?;
        for row_idx in 0..self.total_rows {
            let mut record = Vec::new();
            for col_idx in 0..self.total_cols {
                let index = (row_idx as u32) * (self.total_cols as u32) + (col_idx as u32);
                let cell = self.sheet.get(&index).cloned().unwrap_or_default();
                let value_str = match cell.value {
                    Valtype::Int(n) => n.to_string(),
                    Valtype::Str(s) => s.to_string(),
                };
                record.push(value_str);
            }
            writer.write_record(&record)?;
        }
        writer.flush()?;
        Ok(())
    }
    
    pub fn export_formulas_to_csv(&self, filename: &str) -> Result<(), Box<dyn Error>> {
        let mut writer = Writer::from_path(filename)?;
        for row_idx in 0..self.total_rows {
            let mut record = Vec::new();
            for col_idx in 0..self.total_cols {
                let formula = self.get_cell_formula(row_idx, col_idx);
                record.push(formula);
            }
            writer.write_record(&record)?;
        }
        writer.flush()?;
        Ok(())
    }
 
}
