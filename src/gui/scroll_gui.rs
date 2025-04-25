use crate::{
    gui::gui_defs::{Direction, SpreadsheetApp},
    gui::utils_gui::col_label,
};

/// Moves the view upward by a specified amount.
///
/// If the amount exceeds the current position, it moves to the top (row 0).
///
/// # Arguments
/// * `start_row` - A mutable reference to the current starting row index.
/// * `amount` - The number of rows to move upward.
pub fn w(start_row: &mut usize, amount: usize) {
    if *start_row >= amount {
        *start_row -= amount;
    } else {
        *start_row = 0;
    }
}

/// Moves the view downward by a specified amount.
///
/// If the movement would exceed the total rows, it moves to the bottom limit.
///
/// # Arguments
/// * `start_row` - A mutable reference to the current starting row index.
/// * `total_rows` - The total number of rows in the spreadsheet.
/// * `amount` - The number of rows to move downward.
pub fn s(start_row: &mut usize, total_rows: usize, amount: usize) {
    if *start_row + amount <= total_rows - amount {
        *start_row += amount;
    } else if *start_row >= total_rows - amount {
        // Do nothing, already at or past the end
    } else {
        *start_row = total_rows - amount;
    }
}

/// Moves the view leftward by a specified amount.
///
/// If the amount exceeds the current position, it moves to the leftmost column (column 0).
///
/// # Arguments
/// * `start_col` - A mutable reference to the current starting column index.
/// * `amount` - The number of columns to move leftward.
pub fn a(start_col: &mut usize, amount: usize) {
    if *start_col >= amount {
        *start_col -= amount;
    } else {
        *start_col = 0;
    }
}

/// Moves the view rightward by a specified amount.
///
/// If the movement would exceed the total columns, it moves to the rightmost limit.
///
/// # Arguments
/// * `start_col` - A mutable reference to the current starting column index.
/// * `total_cols` - The total number of columns in the spreadsheet.
/// * `amount` - The number of columns to move rightward.
pub fn d(start_col: &mut usize, total_cols: usize, amount: usize) {
    if *start_col + amount <= total_cols - amount {
        *start_col += amount;
    } else if *start_col >= total_cols - amount {
        // Do nothing, already at or past the end
    } else {
        *start_col = total_cols - amount;
    }
}

impl SpreadsheetApp {
    /// Moves the selection in the specified direction by a given amount.
    ///
    /// Updates the view and status message based on the new position.
    ///
    /// # Arguments
    /// * `direction` - The direction to move (`Up`, `Down`, `Left`, or `Right`).
    /// * `amount` - The number of cells to move in the specified direction.
    pub(in crate::gui) fn move_selection_n(&mut self, direction: Direction, amount: usize) {
        let total_rows = self.total_rows;
        let total_cols = self.total_cols;
        match direction {
            Direction::Up => w(&mut self.start_row, amount),
            Direction::Down => s(&mut self.start_row, total_rows, amount),
            Direction::Right => d(&mut self.start_col, total_cols, amount),
            Direction::Left => a(&mut self.start_col, amount),
        };
        self.status_message = format!(
            "Moved to cell {}{}",
            col_label(self.start_col),
            (self.start_row + 1)
        );
    }
}
