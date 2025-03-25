use std::{cmp::{max, min}, f64, thread::sleep, time::Duration};
use crate::{Cell, CellValue};
use crate::parser::{detect_formula, FormulaType};

pub static mut EVAL_ERROR: bool = false;
pub static mut STATUS_CODE: usize = 0;
pub fn to_indices(s: &str) -> (usize, usize) {
    let split_pos = s.find(|c: char| c.is_ascii_digit()).unwrap_or(s.len());
    let col = s[..split_pos]
        .bytes()
        .fold(0, |acc, b| acc * 26 + (b - b'A' + 1) as usize);
    let row = s[split_pos..].parse::<usize>().unwrap_or(1);
    if row == 0 || col == 0 {
        unsafe { STATUS_CODE = 1; }
        return (0, 0);
    }
    (row - 1, col - 1)
}

pub fn compute(a: i32, op: Option<char>, b: i32) -> i32 {
    match op {
        Some('+') => a + b,
        Some('-') => a - b,
        Some('*') => a * b,
        Some('/') => {
            if b == 0 {
                unsafe { EVAL_ERROR = true; }
                0
            } else {
                a / b
            }
        }
        _ => {
            unsafe { STATUS_CODE = 2; }
            0
        }
    }
}
pub fn sleepy(x: i32) {
    if x > 0 {
        sleep(Duration::from_secs(x as u64));
    }
}
pub fn compute_range(sheet: &Vec<Vec<Cell>>, r_min: usize, r_max: usize, c_min: usize, c_max: usize, choice: i32) -> i32 {
    let width = (c_max - c_min + 1) as i32;
    let height = (r_max - r_min + 1) as i32;
    let area = width * height;
    let mut res: i32 = match choice {
        1 => i32::MIN,
        2 => i32::MAX,
        _ => 0,
    };
    let mut variance: f64 = 0.0;
    for r in r_min..=r_max {
        for c in c_min..=c_max {
            match &sheet[r][c].value {
                CellValue::Str(_) => {
                    unsafe { EVAL_ERROR = true; }
                    return 0;
                }
                CellValue::Int(val) => match choice {
                    1 => res = max(res, *val),
                    2 => res = min(res, *val),
                    3 | 4 | 5 => res += *val,
                    _ => unsafe { STATUS_CODE = 2; },
                },
            }
        }
    }
    if choice == 3 {
        return res / area;
    }
    if choice == 5 {
        let n = area;
        let mean = res / n;
        for r in r_min..=r_max {
            for c in c_min..=c_max {
                if let CellValue::Int(val) = sheet[r][c].value {
                    variance += ((val - mean) as f64).powi(2);
                }
            }
        }
        variance /= n as f64;
        return variance.sqrt().round() as i32;
    }
    res
}

pub fn remove_reference(dep_cell: &mut Cell, target: (usize, usize)) {
    let target_u8 = (target.0 as u8, target.1 as u8);
    dep_cell.dependents.remove(&target_u8);
}

pub fn add_range_dependencies(sheet: &mut Vec<Vec<Cell>>, start_ref: &str, end_ref: &str, r: usize, c: usize) {
    let (start_row, start_col) = to_indices(start_ref);
    let (end_row, end_col) = to_indices(end_ref);
    let min_row = start_row.min(end_row);
    let max_row = start_row.max(end_row);
    let min_col = start_col.min(end_col);
    let max_col = start_col.max(end_col);
    for row in min_row..=max_row {
        for col in min_col..=max_col {
            sheet[row][col].dependents.insert((r as u8, c as u8));
        }
    }
}
pub fn detect_cycle(sheet: &Vec<Vec<Cell>>, row: usize, col: usize, visited: &mut [u8], total_rows: usize, total_cols: usize) -> bool {
    let idx = row * total_cols + col;
    if visited[idx] == 1 { return true; }
    if visited[idx] == 2 { return false; }
    visited[idx] = 1;
    if let Some(formula) = &sheet[row][col].formula {
        let parsed = detect_formula(formula);
        match parsed.formula_type {
            FormulaType::Reference | FormulaType::SleepRef | FormulaType::ReferenceConstant => {
                if let Some(ref r1) = parsed.ref1 {
                    let (ref_row, ref_col) = to_indices(r1);
                    if detect_cycle(sheet, ref_row, ref_col, visited, total_rows, total_cols) {
                        return true;
                    }
                }
            }
            FormulaType::ConstantReference => {
                if let Some(ref r2) = parsed.ref2 {
                    let (ref_row, ref_col) = to_indices(r2);
                    if detect_cycle(sheet, ref_row, ref_col, visited, total_rows, total_cols) {
                        return true;
                    }
                }
            }
            FormulaType::ReferenceReference => {
                if let Some(ref r1) = parsed.ref1 {
                    let (ref_row, ref_col) = to_indices(r1);
                    if detect_cycle(sheet, ref_row, ref_col, visited, total_rows, total_cols) {
                        return true;
                    }
                }
                if let Some(ref r2) = parsed.ref2 {
                    let (ref_row, ref_col) = to_indices(r2);
                    if detect_cycle(sheet, ref_row, ref_col, visited, total_rows, total_cols) {
                        return true;
                    }
                }
            }
            FormulaType::RangeFunction => {
                if let (Some(ref r1), Some(ref r2)) = (parsed.ref1, parsed.ref2) {
                    let (start_row, start_col) = to_indices(r1);
                    let (end_row, end_col) = to_indices(r2);
                    let (min_row, max_row) = (start_row.min(end_row), start_row.max(end_row));
                    let (min_col, max_col) = (start_col.min(end_col), start_col.max(end_col));
                    for i in min_row..=max_row {
                        for j in min_col..=max_col {
                            if detect_cycle(sheet, i, j, visited, total_rows, total_cols) {
                                return true;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    visited[idx] = 2;
    false
}

pub fn run_cycle_detection(sheet: &Vec<Vec<Cell>>, start_row: usize, start_col: usize, total_rows: usize, total_cols: usize, visited: &mut Vec<u8>) -> bool {
    visited.fill(0);
    detect_cycle(sheet, start_row, start_col, visited, total_rows, total_cols)
}

pub fn update_cell(sheet: &mut Vec<Vec<Cell>>, total_rows: usize, total_cols: usize, r: usize, c: usize, formula: &str, visited: &mut Vec<u8>) {
    let backup = sheet[r][c].formula.clone();
    let parsed = detect_formula(formula);
    match parsed.formula_type {
        FormulaType::Reference | FormulaType::SleepRef | FormulaType::ReferenceConstant => {
            if let Some(ref r1) = parsed.ref1 {
                let (row, col) = to_indices(r1);
                if row >= total_rows || col >= total_cols { unsafe { STATUS_CODE = 1; }return; }
            }
        }
        FormulaType::ConstantReference => {
            if let Some(ref r2) = parsed.ref2 {
                let (row, col) = to_indices(r2);
                if row >= total_rows || col >= total_cols { unsafe { STATUS_CODE = 1; }return; }
            }
        }
        FormulaType::ReferenceReference | FormulaType::RangeFunction => {
            if let Some(ref r1) = parsed.ref1 {
                let (row, col) = to_indices(r1);
                if row >= total_rows || col >= total_cols { unsafe { STATUS_CODE = 1; }return; }
            }
            if let Some(ref r2) = parsed.ref2 {
                let (row, col) = to_indices(r2);
                if row >= total_rows || col >= total_cols { unsafe { STATUS_CODE = 1; }return; }
            }
        }
        FormulaType::InvalidFormula => { unsafe { STATUS_CODE = 2; } return; }
        _ => {}
    }
    if let Some(old_formula) = sheet[r][c].formula.take() {
        let old_parsed = detect_formula(&old_formula);
        match old_parsed.formula_type {
            FormulaType::RangeFunction => {
                if let (Some(ref old_r1), Some(ref old_r2)) = (old_parsed.ref1, old_parsed.ref2) {
                    let (s_row, s_col) = to_indices(old_r1);
                    let (e_row, e_col) = to_indices(old_r2);
                    let (min_row, max_row) = (s_row.min(e_row), s_row.max(e_row));
                    let (min_col, max_col) = (s_col.min(e_col), s_col.max(e_col));
                    for i in min_row..=max_row {
                        for j in min_col..=max_col {
                            remove_reference(&mut sheet[i][j], (r, c));
                        }
                    }
                }
            }
            _ => {
                if let Some(ref old_r1) = old_parsed.ref1 {
                    let (i, j) = to_indices(old_r1);
                    remove_reference(&mut sheet[i][j], (r, c));
                }
                if let Some(ref old_r2) = old_parsed.ref2 {
                    let (i, j) = to_indices(old_r2);
                    remove_reference(&mut sheet[i][j], (r, c));
                }
            }
        }
    }
    sheet[r][c].formula = Some(formula.to_string());
    if run_cycle_detection(sheet, r, c, total_rows, total_cols, visited) {
        unsafe { STATUS_CODE = 3; }
        sheet[r][c].formula = backup;
        return;
    }
    match parsed.formula_type {
        FormulaType::RangeFunction => {
            if let (Some(ref new_r1), Some(ref new_r2)) = (parsed.ref1, parsed.ref2) {
                add_range_dependencies(sheet, new_r1, new_r2, r, c);
            }
        }
        _ => {
            if let Some(ref new_r1) = parsed.ref1 {
                let (dep_row, dep_col) = to_indices(new_r1);
                sheet[dep_row][dep_col].dependents.insert((r as u8, c as u8));
            }
            if let Some(ref new_r2) = parsed.ref2 {
                let (dep_row, dep_col) = to_indices(new_r2);
                sheet[dep_row][dep_col].dependents.insert((r as u8, c as u8));
            }
        }
    }
}
