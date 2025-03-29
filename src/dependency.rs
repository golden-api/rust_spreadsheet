use crate::{Cell, FormulaType,STATUS_CODE,utils::to_indices};

pub fn remove_reference(dep_cell: &mut Cell, target: (usize, usize)) {
    dep_cell.dependents.remove(&target);
}

pub fn add_range_dependencies(sheet: &mut Vec<Vec<Cell>>, start_ref: &str, end_ref: &str, r: usize, c: usize) {
    let (start_row, start_col) = to_indices(start_ref);
    let (end_row, end_col) = to_indices(end_ref);
    for row in start_row..=end_row {
        for col in start_col..=end_col {
            sheet[row][col].dependents.insert((r, c));
        }
    }
}
pub fn detect_cycle(sheet: &Vec<Vec<Cell>>, row: usize, col: usize, visited: &mut [u8], total_rows: usize, total_cols: usize) -> bool {
    let idx = row * total_cols + col;
    if visited[idx] == 1 { return true; }
    if visited[idx] == 2 { return false; }
    visited[idx] = 1;
    let cell = &sheet[row][col];
    if let Some(formula) = &cell.formula {
        match formula {
            FormulaType::Range => {
                if let (Some(r1), Some(r2)) = (&cell.cell1, &cell.cell2) {
                    let (start_row, start_col) = to_indices(r1);
                    let (end_row, end_col) = to_indices(r2);
                    for i in start_row..=end_row {
                        for j in start_col..=end_col {
                            if detect_cycle(sheet, i, j, visited, total_rows, total_cols) {
                                return true;
                            }
                        }
                    }
                }
            }
            _ => {
                if let Some(r1) = &cell.cell1 {
                    let (ref_row, ref_col) = to_indices(r1);
                    if detect_cycle(sheet, ref_row, ref_col, visited, total_rows, total_cols) {
                        return true;
                    }
                }
                if let Some(r2) = &cell.cell2 {
                    let (ref_row, ref_col) = to_indices(r2);
                    if detect_cycle(sheet, ref_row, ref_col, visited, total_rows, total_cols) {
                        return true;
                    }
                }
            }
        }
    }
    visited[idx] = 2;
    false
}

pub fn run_cycle_detection(sheet: &Vec<Vec<Cell>>, start_row: usize, start_col: usize, total_rows: usize, total_cols: usize, visited: &mut Vec<u8>) -> bool {
    visited.fill(0);
    detect_cycle(sheet, start_row, start_col, visited, total_rows, total_cols)
}

pub fn update_cell(sheet: &mut Vec<Vec<Cell>>, total_rows: usize, total_cols: usize, r: usize, c: usize, visited: &mut Vec<u8>, mut backup : Cell) {
    {
        match &sheet[r][c].formula {
            Some(FormulaType::Invalid) => {
                unsafe { STATUS_CODE = 2; }
                return;
            }
            _ => {
                if let Some(r1) = &sheet[r][c].cell1 {
                    let (row_idx, col_idx) = to_indices(r1);
                    if row_idx >= total_rows || col_idx >= total_cols {
                        unsafe { STATUS_CODE = 1; }
                    }
                }
                if let Some(r2) = &sheet[r][c].cell2 {
                    let (row_idx, col_idx) = to_indices(r2);
                    if row_idx >= total_rows || col_idx >= total_cols {
                        unsafe { STATUS_CODE = 1; }
                    }
                }
            }
        }
    }
    if run_cycle_detection(sheet, r, c, total_rows, total_cols, visited) {
        unsafe { STATUS_CODE = 3; }
        std::mem::swap(&mut backup.dependents, &mut sheet[r][c].dependents);
        sheet[r][c] = backup;
        return;
    }
    {
        match &backup.formula {
        Some(FormulaType::Range) => {
            if let (Some(old_r1), Some(old_r2)) =
                (backup.cell1.as_ref(), backup.cell2.as_ref())
            {
                let (start_row, start_col) = to_indices(old_r1);
                let (end_row, end_col) = to_indices(old_r2);
                for i in start_row..=end_row {
                    for j in start_col..=end_col {
                        remove_reference(&mut sheet[i][j], (r, c));
                    }
                }
            }
        }
        _ => {
            if let Some(old_r1) = backup.cell1.as_ref() {
                let (i, j) = to_indices(old_r1);
                remove_reference(&mut sheet[i][j], (r, c));
            }
            if let Some(old_r2) = backup.cell2.as_ref() {
                let (i, j) = to_indices(old_r2);
                remove_reference(&mut sheet[i][j], (r, c));
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
                    add_range_dependencies(sheet, start, end, r, c);
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
