use crate::{Cell, FormulaType, STATUS_CODE, utils::to_indices};

pub fn update_cell(
    sheet: &mut Vec<Vec<Cell>>,
    total_rows: usize,
    total_cols: usize,
    r: usize,
    c: usize,
    backup: Cell,
) {
    {
        match &sheet[r][c].formula {
            Some(FormulaType::Invalid) => {
                unsafe {
                    STATUS_CODE = 2;
                }
                return;
            }
            _ => {
                if let Some(ref r1) = sheet[r][c].cell1 {
                    let (row_idx, col_idx) = to_indices(r1);
                    if row_idx >= total_rows || col_idx >= total_cols {
                        unsafe {
                            STATUS_CODE = 1;
                        }
                    }
                }
                if let Some(ref r2) = sheet[r][c].cell2 {
                    let (row_idx, col_idx) = to_indices(r2);
                    if row_idx >= total_rows || col_idx >= total_cols {
                        unsafe {
                            STATUS_CODE = 1;
                        }
                    }
                }
            }
        }
    }
    if unsafe { STATUS_CODE } != 0 {
        return;
    }

    {
        match &backup.formula {
            Some(FormulaType::Range) => {
                if let (Some(old_r1), Some(old_r2)) = (backup.cell1.as_ref(), backup.cell2.as_ref())
                {
                    let (start_row, start_col) = to_indices(old_r1);
                    let (end_row, end_col) = to_indices(old_r2);
                    for i in start_row..=end_row {
                        for j in start_col..=end_col {
                            sheet[i][j].dependents.remove(&(r, c));
                        }
                    }
                }
            }
            _ => {
                if let Some(old_r1) = backup.cell1.as_ref() {
                    let (i, j) = to_indices(old_r1);
                    sheet[i][j].dependents.remove(&(r, c));
                }
                if let Some(old_r2) = backup.cell2.as_ref() {
                    let (i, j) = to_indices(old_r2);
                    sheet[i][j].dependents.remove(&(r, c));
                }
            }
        }
    }

    {
        match &sheet[r][c].formula {
            Some(FormulaType::Range) => {
                let new_r1 = sheet[r][c].cell1.clone();
                let new_r2 = sheet[r][c].cell2.clone();
                if let (Some(ref start), Some(ref end)) = (new_r1, new_r2) {
                    let (start_row, start_col) = to_indices(start);
                    let (end_row, end_col) = to_indices(end);
                    for row in start_row..=end_row {
                        for col in start_col..=end_col {
                            sheet[row][col].dependents.insert((r, c));
                        }
                    }
                }
            }
            _ => {
                if let Some(new_r1) = &sheet[r][c].cell1 {
                    let (dep_row, dep_col) = to_indices(new_r1);
                    sheet[dep_row][dep_col].dependents.insert((r, c));
                }
                if let Some(new_r2) = &sheet[r][c].cell2 {
                    let (dep_row, dep_col) = to_indices(new_r2);
                    sheet[dep_row][dep_col].dependents.insert((r, c));
                }
            }
        }
    }
}
