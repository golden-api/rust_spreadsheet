//! # Utils Module
//! This module provides utility functions for the spreadsheet application,
//! including cell reference conversion, arithmetic operations, range computations,
//! and helper functions for dependency management.
use std::{collections::HashMap, f64, thread::sleep, time::Duration};

use crate::{Cell, Valtype, STATUS_CODE};

/// A global flag indicating if an evaluation error occurred.
pub static mut EVAL_ERROR: bool = false;



/// Converts a cell reference (e.g., "A1") to row and column indices (0-based).
///
/// # Arguments
/// * `s` - The cell reference string.
///
/// # Returns
/// A tuple `(usize, usize)` representing (row, column) indices.
///
/// # Panics
/// Panics if the string format is invalid (should be handled by caller).
///
/// # Examples
/// ```
/// let (row, col) = to_indices("A1");
/// assert_eq!((row, col), (0, 0));
/// ```
pub fn to_indices(s: &str) -> (usize, usize) {
    let split_pos = s.find(|c: char| c.is_ascii_digit()).unwrap_or(s.len());
    let col = s[..split_pos]
        .bytes()
        .fold(0, |acc, b| acc * 26 + (b - b'A' + 1) as usize);
    let row = s[split_pos..].parse::<usize>().unwrap_or(0);
    if row == 0 || col == 0 {
        unsafe {
            STATUS_CODE = 1;
        }
        return (0, 0);
    }
    (row - 1, col - 1)
}

/// Performs a binary arithmetic operation on two integers.
///
/// # Arguments
/// * `a` - The first operand.
/// * `op` - The optional operation (e.g., '+', '-', '*', '/').
/// * `b` - The second operand.
///
/// # Returns
/// The result of the operation as an `i32`.
///
/// # Examples
/// ```
/// let result = compute(5, Some('+'), 3);
/// assert_eq!(result, 8);
/// ```
pub fn compute(a: i32, op: Option<char>, b: i32) -> i32 {
    match op {
        Some('+') => a + b,
        Some('-') => a - b,
        Some('*') => a * b,
        Some('/') => {
            if b == 0 {
                unsafe {
                    EVAL_ERROR = true;
                }
                0
            } else {
                a / b
            }
        }
        _ => {
            unsafe {
                STATUS_CODE = 2;
            }
            0
        }
    }
}

/// Simulates a sleep operation for the given number of seconds.
///
/// # Arguments
/// * `x` - The number of seconds to sleep (non-negative).
pub fn sleepy(x: i32) {
    if x > 0 {
        sleep(Duration::from_secs(x as u64))
    }
}

/// Compute MIN, MAX, SUM, AVG, or STDEV over a rectangular block in a sparse sheet.
///
/// # Arguments
/// * `sheet` - A hash map containing cell data, indexed by a unique `u32` key.
/// * `total_cols` - The total number of columns in the spreadsheet.
/// * `r_min` - The minimum row index of the range.
/// * `r_max` - The maximum row index of the range.
/// * `c_min` - The minimum column index of the range.
/// * `c_max` - The maximum column index of the range.
/// * `choice` - The function to apply (1=MAX, 2=MIN, 3=AVG, 4=SUM, 5=STDEV).
///
/// # Returns
/// The computed result as an `i32`.
///
/// # Examples
/// ```
/// let mut sheet: HashMap<u32, Cell> = HashMap::new();
/// sheet.insert(0, Cell { value: Valtype::Int(5), data: CellData::Const, dependents: HashSet::new() });
/// let result = compute_range(&sheet, 10, 0, 0, 0, 0, 4); // SUM
/// assert_eq!(result, 5);
/// ```

pub fn compute_range(
    sheet: &HashMap<u32, Cell>,
    total_cols: usize,
    r_min: usize,
    r_max: usize,
    c_min: usize,
    c_max: usize,
    choice: i32,
) -> i32 {
    let width = (c_max - c_min + 1) as usize;
    let height = (r_max - r_min + 1) as usize;
    let area = width * height;
    let use_hashmap_iter = sheet.len() >= area;
    // If area is small, do the simple full scan:
    if use_hashmap_iter {
        // --- original version ---
        let mut res: i32 = match choice {
            1 => i32::MIN, // MAX
            2 => i32::MAX, // MIN
            _ => 0,        // SUM/AVG/STDEV
        };
        let mut variance = 0.0;

        for rr in r_min..=r_max {
            for cc in c_min..=c_max {
                let key = (rr * total_cols + cc) as u32;
                let val = match sheet
                    .get(&key)
                    .map(|c| &c.value)
                    .unwrap_or(&Valtype::Int(0))
                {
                    Valtype::Int(v) => *v,
                    Valtype::Str(_) => { unsafe { EVAL_ERROR = true; } continue; }
                };
                match choice {
                    1 => res = res.max(val),
                    2 => res = res.min(val),
                    3..=5 => res += val,
                    _ => unsafe { STATUS_CODE = 2; },
                }
            }
        }

        match choice {
            3 => res / (area as i32),                    // AVG
            5 => {  // STDEV: second-pass
                let mean = res as f64 / area as f64;
                for rr in r_min..=r_max {
                    for cc in c_min..=c_max {
                        let key = (rr * total_cols + cc) as u32;
                        if let Some(Valtype::Int(v)) = sheet.get(&key).map(|c| c.value.clone()) {
                            variance += (v as f64 - mean).powi(2);
                        } else {
                            variance += (0.0 - mean).powi(2);
                        }
                    }
                }
                (variance / area as f64).sqrt().round() as i32
            }
            _ => res,
        }
    } else {
        // --- optimized sparse scan ---
        // Track number of entries seen in-range:
        let mut count_in = 0usize;
        // accumulators:
        let mut max_v = i32::MIN;
        let mut min_v = i32::MAX;
        let mut sum = 0i32;  // use i64 to avoid overflow on large areas
        let mut variance_acc = 0.0;

        // First pass: only look at the non-zero cells we actually stored
        for (&key, cell) in sheet.iter() {
            let row = (key as usize) / total_cols;
            let col = (key as usize) % total_cols;
            if row < r_min || row > r_max || col < c_min || col > c_max {
                continue;
            }
            let v = match &cell.value {
                Valtype::Int(v) => *v as i32,
                Valtype::Str(_) => { unsafe { EVAL_ERROR = true; } continue; }
            };
            count_in += 1;
            sum += v;
            max_v = max_v.max(v as i32);
            min_v = min_v.min(v as i32);
        }

        let zero_count = area.saturating_sub(count_in);
        match choice {
            1 => {
                // MAX: if any zeros were omitted, they could be the max
                if zero_count > 0 {
                    max_v = max_v.max(0);
                }
                max_v
            }
            2 => {
                // MIN: zeros could be the min if no negatives
                if zero_count > 0 {
                    min_v = min_v.min(0);
                }
                min_v
            }
            4 => {
                // SUM: zeros don't change sum
                sum as i32
            }
            3 => {
                // AVG: include zeros
                sum / (area as i32)
            }
            5 => {
                // STDEV:
                let mean = sum as f64 / area as f64;
                // variance contribution from non-zero cells:
                for (&key, cell) in sheet.iter() {
                    let row = (key as usize) / total_cols;
                    let col = (key as usize) % total_cols;
                    if row < r_min || row > r_max || col < c_min || col > c_max {
                        continue;
                    }
                    if let Valtype::Int(v) = cell.value {
                        variance_acc += (v as f64 - mean).powi(2);
                    }
                }
                // variance contribution from zeros:
                variance_acc += (zero_count as f64) *( (0.0 - mean).powi(2));

                (variance_acc / area as f64).sqrt().round() as i32
            }
            _ => {
                unsafe { STATUS_CODE = 2; }
                0
            }
        }
    }
}


/// Checks if a cell index falls within a given range.
///
/// # Arguments
/// * `idx` - The cell index to check.
/// * `start` - The starting cell index of the range.
/// * `end` - The ending cell index of the range.
/// * `total_cols` - The total number of columns in the spreadsheet.
///
/// # Returns
/// * `bool` - `true` if the index is within the range, `false` otherwise.
pub fn in_range(idx: u32, start: u32, end: u32, total_cols: usize) -> bool {
    let (r0, c0) = (idx as usize / total_cols, idx as usize % total_cols);
    let (sr, sc) = (start as usize / total_cols, start as usize % total_cols);
    let (er, ec) = (end as usize / total_cols, end as usize % total_cols);
    (sr <= r0 && r0 <= er) && (sc <= c0 && c0 <= ec)
}
