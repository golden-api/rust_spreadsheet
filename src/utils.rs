use std::{cmp::{max, min}, f64, thread::sleep, time::Duration};
use crate::{Cell, CellValue};
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
pub fn compute_range(
    sheet: &Vec<Vec<Cell>>,
    r_min: usize,
    r_max: usize,
    c_min: usize,
    c_max: usize,
    choice: i32,
) -> i32 {
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