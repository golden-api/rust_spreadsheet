use crate::{STATUS_CODE, utils::to_indices};

/// # Scrolling Module
/// This module provides functions to manage scrolling within the spreadsheet grid,
/// allowing navigation through rows and columns using keyboard-like commands
/// (e.g., 'w' for up, 's' for down, 'a' for left, 'd' for right) and direct cell targeting.

/// Moves the view up by 10 rows if possible.
///
/// # Arguments
/// * `start_row` - A mutable reference to the current starting row index.
pub fn w(start_row: &mut usize) {
    if *start_row >= 10 {
        *start_row -= 10;
    } else {
        *start_row = 0;
    }
}

/// Moves the view down by 10 rows if possible.
///
/// # Arguments
/// * `start_row` - A mutable reference to the current starting row index.
/// * `total_rows` - The total number of rows in the spreadsheet.
pub fn s(start_row: &mut usize, total_rows: usize) {
    if *start_row + 10 <= total_rows - 10 {
        *start_row += 10;
    } else if *start_row >= total_rows - 10 {
        *start_row += 0;
    } else {
        *start_row = total_rows - 10;
    }
}

/// Moves the view left by 10 columns if possible.
///
/// # Arguments
/// * `start_col` - A mutable reference to the current starting column index.
pub fn a(start_col: &mut usize) {
    if *start_col >= 10 {
        *start_col -= 10;
    } else {
        *start_col = 0;
    }
}

/// Moves the view right by 10 columns if possible.
///
/// # Arguments
/// * `start_col` - A mutable reference to the current starting column index.
/// * `total_cols` - The total number of columns in the spreadsheet.
pub fn d(start_col: &mut usize, total_cols: usize) {
    if *start_col + 10 <= total_cols - 10 {
        *start_col += 10;
    } else if *start_col >= total_cols - 10 {
        *start_col += 0;
    } else {
        *start_col = total_cols - 10;
    }
}

/// Scrolls the view to a specific cell reference.
///
/// # Arguments
/// * `start_row` - A mutable reference to the current starting row index.
/// * `start_col` - A mutable reference to the current starting column index.
/// * `total_rows` - The total number of rows in the spreadsheet.
/// * `total_cols` - The total number of columns in the spreadsheet.
/// * `cell_ref` - The cell reference to scroll to (e.g., "A1").
///
/// # Returns
/// * `Result<(), ()>` - `Ok(())` on success, `Err(())` if the reference is invalid or out of bounds.
///
/// # Examples
/// ```
/// let mut row = 0;
/// let mut col = 0;
/// let result = scroll_to(&mut row, &mut col, 10, 10, "B2");
/// assert!(result.is_ok());
/// ```
pub fn scroll_to(
    start_row: &mut usize,
    start_col: &mut usize,
    total_rows: usize,
    total_cols: usize,
    cell_ref: &str,
) -> Result<(), ()> {
    let (row, col) = to_indices(cell_ref);
    if row >= total_rows || col >= total_cols || unsafe { STATUS_CODE } == 1 {
        return Err(());
    }
    *start_row = row;
    *start_col = col;
    Ok(())
}
