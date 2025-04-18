use crate::{gui_defs::{Direction, SpreadsheetApp}, utils_gui::col_label};

pub fn w(
    start_row: &mut usize,
    amount: usize,
) {
    if *start_row >= amount {
        *start_row -= amount;
    } else {
        *start_row = 0;
    }
}


pub fn s(
    start_row: &mut usize,
    total_rows: usize,
    amount: usize,
) {
    if *start_row + amount <= total_rows - amount {
        *start_row += amount;
    } else if *start_row >= total_rows - amount {
        // Do nothing, already at or past the end
    } else {
        *start_row = total_rows - amount;
    }
}


pub fn a(
    start_col: &mut usize,
    amount: usize,
) {
    if *start_col >= amount {
        *start_col -= amount;
    } else {
        *start_col = 0;
    }
}


pub fn d(
    start_col: &mut usize,
    total_cols: usize,
    amount: usize,
) {
    if *start_col + amount <= total_cols - amount {
        *start_col += amount;
    } else if *start_col >= total_cols - amount {
        // Do nothing, already at or past the end
    } else {
        *start_col = total_cols - amount;
    }
}

impl SpreadsheetApp{
    pub fn move_selection_n(
        &mut self,
        direction: Direction,
        amount: usize,
    ) {
        let total_rows = self.sheet.len();
        let total_cols = self.sheet[0].len();
        match direction {
            Direction::Up => w(&mut self.start_row, amount),
            Direction::Down => s(&mut self.start_row, total_rows, amount),
            Direction::Right => d(&mut self.start_col, total_cols, amount),
            Direction::Left => a(&mut self.start_row, amount),
        };
        self.status_message = format!("Moved to cell {}{}", col_label(self.start_col), (self.start_row + 1).to_string());
    }

    
}