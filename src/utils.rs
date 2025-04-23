use std::{
    collections::HashMap,
    f64,
    thread::sleep,
    time::Duration,
};

use crate::{
    Cell,
    STATUS_CODE,
    Valtype,
};

pub static mut EVAL_ERROR: bool = false;

pub fn to_indices(s: &str) -> (usize, usize) {
    let split_pos = s.find(|c: char| c.is_ascii_digit()).unwrap_or(s.len());
    let col = s[..split_pos].bytes().fold(0, |acc, b| acc * 26 + (b - b'A' + 1) as usize);
    let row = s[split_pos..].parse::<usize>().unwrap_or(0);
    if row == 0 || col == 0 {
        unsafe {
            STATUS_CODE = 1;
        }
        return (0, 0);
    }
    (row - 1, col - 1)
}

pub fn compute(
    a: i32,
    op: Option<char>,
    b: i32,
) -> i32 {
    match op {
        Some('+') => a + b,
        Some('-') => a - b,
        Some('*') => a * b,
        Some('/') =>
            if b == 0 {
                unsafe {
                    EVAL_ERROR = true;
                }
                0
            } else {
                a / b
            },
        _ => {
            unsafe {
                STATUS_CODE = 2;
            }
            0
        }
    }
}
pub fn sleepy(x: i32) {
    if x > 0 {
        sleep(Duration::from_secs(x as u64))
    }
}
/// Compute MIN, MAX, SUM, AVG, or STDEV over a rectangular block in a sparse sheet.
pub fn compute_range(
    sheet: &HashMap<u32, Cell>,
    total_cols: usize,
    r_min: usize,
    r_max: usize,
    c_min: usize,
    c_max: usize,
    choice: i32,
) -> i32 {
    let width  = (c_max - c_min + 1) as i32;
    let height = (r_max - r_min + 1) as i32;
    let area   = width * height;

    // initial accumulator
    let mut res: i32 = match choice {
        1 => i32::MIN,  // MAX
        2 => i32::MAX,  // MIN
        _ => 0,         // SUM/AVG/STDEV
    };
    let mut variance: f64 = 0.0;
    // iterate every cell in the rectangle
    for rr in r_min..=r_max {
        for cc in c_min..=c_max {
            let key = (rr * total_cols + cc) as u32;
            let val = match sheet.get(&key).map(|c| &c.value).unwrap_or(&Valtype::Int(0)) {
                Valtype::Int(v) => *v,
                Valtype::Str(_) => {
                    unsafe { EVAL_ERROR = true; }
                    continue;
                }
            };
            match choice {
                1 => res = res.max(val),
                2 => res = res.min(val),
                3 | 4 | 5 => res += val,
                _ => unsafe { STATUS_CODE = 2; }, 
            }
        }
    }

    match choice {
        3 => { // AVG
            res / area
        }
        5 => { // STDEV
            let mean = res as f64 / area as f64;
            // second pass for variance
            for rr in r_min..=r_max {
                for cc in c_min..=c_max {
                    let key = (rr * total_cols + cc) as u32;
                    if let Some(Valtype::Int(v)) = sheet.get(&key).map(|c| c.value.clone()) {
                        variance += (v as f64 - mean).powi(2);
                    }
                }
            }
            variance = variance / area as f64;
            variance.sqrt().round() as i32
        }
        _ => res, // SUM (4) or if choice was 1/2
    }
}
pub fn in_range(idx: u32, start: u32, end: u32, total_cols: usize) -> bool {
    let (r0, c0) = (idx as usize / total_cols, idx as usize % total_cols);
    let (sr, sc) = (start as usize / total_cols, start as usize % total_cols);
    let (er, ec) = (end as usize / total_cols, end as usize % total_cols);
    (sr <= r0 && r0 <= er) && (sc <= c0 && c0 <= ec)
}