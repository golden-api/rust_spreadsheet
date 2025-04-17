#[cfg(feature = "gui")]
// Helper: Convert column index to Excel-style label (A, B, …, Z, AA, etc.)
pub fn col_label(mut col_index: usize) -> String {
    let mut name = String::new();
    loop {
        let remainder = col_index % 26;
        name.insert(0, (b'A' + remainder as u8) as char);
        if col_index < 26 {
            break;
        }
        col_index = col_index / 26 - 1;
    }
    name
}
#[cfg(feature = "gui")]
pub fn parse_cell_name(name: &str) -> Option<(usize, usize)> {
    let mut col_part = String::new();
    let mut row_part = String::new();
    for c in name.chars() {
        if c.is_ascii_alphabetic() {
            col_part.push(c.to_ascii_uppercase());
        } else if c.is_ascii_digit() {
            row_part.push(c);
        } else {
            return None;
        }
    }
    if col_part.is_empty() || row_part.is_empty() {
        return None;
    }
    let col_index = col_label_to_index(&col_part)?;
    let row_index = row_part.parse::<usize>().ok()?.saturating_sub(1);
    Some((row_index, col_index))
}
#[cfg(feature = "gui")]
pub fn col_label_to_index(label: &str) -> Option<usize> {
    let mut col = 0;
    for (i, c) in label.chars().rev().enumerate() {
        if !c.is_ascii_uppercase() {
            return None;
        }
        col += ((c as u8 - b'A') as usize + 1) * 26_usize.pow(i as u32);
    }
    Some(col - 1)
}
#[cfg(feature = "gui")]
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

#[cfg(feature = "gui")]
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

#[cfg(feature = "gui")]
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

#[cfg(feature = "gui")]
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
