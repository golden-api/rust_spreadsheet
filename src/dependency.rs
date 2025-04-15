use crate::{Cell, CellData, STATUS_CODE, utils::to_indices};

pub fn update_cell(
    sheet: &mut Vec<Vec<Cell>>,
    total_rows: usize,
    total_cols: usize,
    r: usize,
    c: usize,
    backup: Cell,
) {
    {
        match &sheet[r][c].data {
            CellData::Invalid => {
                unsafe { STATUS_CODE = 2; }
                return;
            }
            _ => {
                match &sheet[r][c].data {
                    CellData::Range { cell1, cell2, .. } => {
                        let (row_idx, col_idx) = to_indices(cell1);
                        if row_idx >= total_rows || col_idx >= total_cols {
                            unsafe { STATUS_CODE = 1; }
                        }
                        let (row_idx2, col_idx2) = to_indices(cell2);
                        if row_idx2 >= total_rows || col_idx2 >= total_cols {
                            unsafe { STATUS_CODE = 1; }
                        }
                    },
                    CellData::Ref { cell1 } => {
                        let (row_idx, col_idx) = to_indices(cell1);
                        if row_idx >= total_rows || col_idx >= total_cols {
                            unsafe { STATUS_CODE = 1; }
                        }
                    },
                    CellData::CoR { cell2, .. } => {
                        let (row_idx, col_idx) = to_indices(cell2);
                        if row_idx >= total_rows || col_idx >= total_cols {
                            unsafe { STATUS_CODE = 1; }
                        }
                    },
                    CellData::RoC { cell1, .. } => {
                        let (row_idx, col_idx) = to_indices(cell1);
                        if row_idx >= total_rows || col_idx >= total_cols {
                            unsafe { STATUS_CODE = 1; }
                        }
                    },
                    CellData::RoR { cell1, cell2, .. } => {
                        let (row_idx, col_idx) = to_indices(cell1);
                        if row_idx >= total_rows || col_idx >= total_cols {
                            unsafe { STATUS_CODE = 1; }
                        }
                        let (row_idx2, col_idx2) = to_indices(cell2);
                        if row_idx2 >= total_rows || col_idx2 >= total_cols {
                            unsafe { STATUS_CODE = 1; }
                        }
                    },
                    CellData::SleepR { cell1 } => {
                        let (row_idx, col_idx) = to_indices(cell1);
                        if row_idx >= total_rows || col_idx >= total_cols {
                            unsafe { STATUS_CODE = 1; }
                        }
                    },
                    _ => {} // already handled above.
                }
            }
        }
    }
    if unsafe { STATUS_CODE } != 0 {
        return;
    }

    {
        // Remove old dependencies based on the backup cell's data.
        match backup.data {
            CellData::Range { ref cell1, ref cell2, .. } => {
                let (start_row, start_col) = to_indices(cell1);
                let (end_row, end_col) = to_indices(cell2);
                for i in start_row..=end_row {
                    for j in start_col..=end_col {
                        sheet[i][j].dependents.remove(&(r, c));
                    }
                }
            },
            CellData::Ref { ref cell1 } => {
                let (i, j) = to_indices(cell1);
                sheet[i][j].dependents.remove(&(r, c));
            },
            CellData::CoR { ref cell2, .. } => {
                let (i, j) = to_indices(cell2);
                sheet[i][j].dependents.remove(&(r, c));
            },
            CellData::RoC { ref cell1, .. } => {
                let (i, j) = to_indices(cell1);
                sheet[i][j].dependents.remove(&(r, c));
            },
            CellData::RoR { ref cell1, ref cell2, .. } => {
                let (i, j) = to_indices(cell1);
                sheet[i][j].dependents.remove(&(r, c));
                let (i2, j2) = to_indices(cell2);
                sheet[i2][j2].dependents.remove(&(r, c));
            },
            CellData::SleepR { ref cell1 } => {
                let (i, j) = to_indices(cell1);
                sheet[i][j].dependents.remove(&(r, c));
            },
            _=> {}
        }
    }

    {
        // Clone the cell's data to avoid holding an immutable reference.
        let current_data = sheet[r][c].data.clone();
        // Add new dependencies based on the cloned data.
        match current_data {
            CellData::Range { cell1, cell2, .. } => {
                let (start_row, start_col) = to_indices(&cell1);
                let (end_row, end_col) = to_indices(&cell2);
                for row in start_row..=end_row {
                    for col in start_col..=end_col {
                        sheet[row][col].dependents.insert((r, c));
                    }
                }
            },
            CellData::Ref { cell1 } => {
                let (dep_row, dep_col) = to_indices(&cell1);
                sheet[dep_row][dep_col].dependents.insert((r, c));
            },
            CellData::CoR { cell2, .. } => {
                let (dep_row, dep_col) = to_indices(&cell2);
                sheet[dep_row][dep_col].dependents.insert((r, c));
            },
            CellData::RoC { cell1, .. } => {
                let (dep_row, dep_col) = to_indices(&cell1);
                sheet[dep_row][dep_col].dependents.insert((r, c));
            },
            CellData::RoR { cell1, cell2, .. } => {
                let (dep_row, dep_col) = to_indices(&cell1);
                sheet[dep_row][dep_col].dependents.insert((r, c));
                let (dep_row2, dep_col2) = to_indices(&cell2);
                sheet[dep_row2][dep_col2].dependents.insert((r, c));
            },
            CellData::SleepR { cell1 } => {
                let (dep_row, dep_col) = to_indices(&cell1);
                sheet[dep_row][dep_col].dependents.insert((r, c));
            },
            _ => {}
        }
    }        
}
