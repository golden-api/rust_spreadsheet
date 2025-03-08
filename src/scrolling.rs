use crate::utils::to_indices;

pub fn w(start_row: &mut usize) {
    if *start_row >= 10 {
        *start_row -= 10;
    } else {
        *start_row = 0;
    }
}

pub fn s(start_row: &mut usize, total_rows: usize) {
    if *start_row +10 <= total_rows- 10 {
        *start_row+=10;
    }else if *start_row  >= total_rows - 10{
        *start_row +=0;
    }else {
        *start_row = total_rows - 10;
    }
}

pub fn a(start_col: &mut usize) {
    if *start_col >= 10 {
        *start_col -= 10;
    } else {
        *start_col = 0;
    }
}

pub fn d(start_col: &mut usize, total_cols: usize) {
    if *start_col + 10 <= total_cols - 10 {
        *start_col += 10;
    }else if *start_col  >= total_cols - 10{
        *start_col+=0;
    }else {
        *start_col = total_cols - 10;
    }
}

pub fn scroll_to(start_row: &mut usize,start_col: &mut usize,total_rows: usize,total_cols: usize,cell_ref: &str,) -> Result<(), ()> {
    let (row, col) = to_indices(cell_ref);
    if row >= total_rows || col >= total_cols {
        return Err(());
    }
    *start_row = row;
    *start_col = col;
    Ok(())
}
