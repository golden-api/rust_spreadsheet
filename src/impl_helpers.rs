use std::fs::File;

use csv::Writer;

use crate::{CellData, STATUS, STATUS_CODE, Valtype, gui_defs::SpreadsheetApp, parser, utils_gui::col_label};

impl SpreadsheetApp {
    // Helper: Extract formula from cell
    pub fn get_cell_formula(
        &self,
        row: usize,
        col: usize,
    ) -> String {
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
                    if let Valtype::Int(val2) = value2 { format!("{}{}{}", val1, op_code, val2) } else { String::new() }
                } else {
                    String::new()
                }
            }

            CellData::CoR { op_code, value2, cell2 } => {
                if let Valtype::Int(val1) = value2 {
                    format!("{}{}{}", val1, op_code, cell2.as_str())
                } else {
                    String::new()
                }
            }

            CellData::RoC { op_code, value2, cell1 } => {
                if let Valtype::Int(val2) = value2 {
                    format!("{}{}{}", cell1.as_str(), op_code, val2)
                } else {
                    String::new()
                }
            }

            CellData::RoR { op_code, cell1, cell2 } => {
                format!("{}{}{}", cell1, op_code, cell2)
            }

            CellData::Range { cell1, cell2, value2 } => {
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
    pub fn update_selected_cell(&mut self) {
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
    
    pub fn export_to_csv(
        &mut self,
        filename: &str,
    ) {
        let filename = if filename.ends_with(".csv") { filename.to_string() } else { format!("{}.csv", filename) };

        match File::create(&filename) {
            Ok(file) => {
                let mut wtr = Writer::from_writer(file);
                for row in &self.sheet {
                    let record: Vec<String> = row
                        .iter()
                        .map(|cell| match &cell.value {
                            Valtype::Int(n) => n.to_string(),
                            Valtype::Str(s) => s.to_string(),
                        })
                        .collect();

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

    // Handle cell selection changes
    pub fn handle_selection_change(
        &mut self,
        new_selection: Option<(usize, usize)>,
    ) {
        if let Some((i, j)) = new_selection {
            self.selected = Some((i, j));
            self.formula_input = self.get_cell_formula(i, j);
            self.status_message = format!("Selected cell {}{}", col_label(j), i + 1);
        }
    }

    pub fn goto_cell(
        &mut self,
        cell_ref: &str,
    ) {
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
}
