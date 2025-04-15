use crate::{Cell, CellData,STATUS_CODE, Valtype};
use crate::utils::{EVAL_ERROR, compute, compute_range, sleepy, to_indices};
use std::collections::{HashMap, VecDeque};

use regex::Regex;

pub fn detect_formula(block: &mut Cell, form: &str) {
    let form = form.trim();

    // 1. SLEEP_CONST: "SLEEP(<int>)"
    let re_sleep_const = Regex::new(r"^SLEEP\((-?\d+)\)$").unwrap();
    if let Some(caps) = re_sleep_const.captures(form) {
        if let Some(m) = caps.get(1) {
            if let Ok(val) = m.as_str().parse::<i32>() {
                block.reset();
                block.value = Valtype::Int(val);
                block.data = CellData::SleepC;
                return;
            }
        }
    }
    // 2. SLEEP_REF: "SLEEP(<ref>)"
    let re_sleep_ref = Regex::new(r"^SLEEP\(([A-Z]+[0-9]+)\)$").unwrap();
    if let Some(caps) = re_sleep_ref.captures(form) {
        if let Some(m) = caps.get(1) {
            block.reset();
            let cell_ref = m.as_str().to_string();
            block.data = CellData::SleepR { cell1: cell_ref };
            return;
        }
    }
    // 3. CONSTANT: a lone integer
    let re_constant = Regex::new(r"^(-?\d+)$").unwrap();
    if let Some(caps) = re_constant.captures(form) {
        if let Some(m) = caps.get(1) {
            if let Ok(val) = m.as_str().parse::<i32>() {
                block.reset();
                block.value = Valtype::Int(val);
                block.data = CellData::Const;
                return;
            }
        }
    }
    // 4. REFERENCE: a cell reference (e.g., "A1")
    let re_reference = Regex::new(r"^([A-Z]+[0-9]+)$").unwrap();
    if let Some(caps) = re_reference.captures(form) {
        if let Some(m) = caps.get(1) {
            block.reset();
            let cell_ref = m.as_str().to_string();
            block.data = CellData::Ref { cell1: cell_ref };
            return;
        }
    }
    // 5. CONSTANT_CONSTANT: "<int><op><int>"
    let re_const_const = Regex::new(r"^(-?\d+)([-+*/])(-?\d+)$").unwrap();
    if let Some(caps) = re_const_const.captures(form) {
        block.reset();
        let val1: i32 = caps.get(1).unwrap().as_str().parse().unwrap();
        let op = caps.get(2).unwrap().as_str().chars().next().unwrap();
        let val2: i32 = caps.get(3).unwrap().as_str().parse().unwrap();
        block.value = Valtype::Int(val1);
        block.data = CellData::CoC { op_code: op, value2: Valtype::Int(val2) };
        return;
    }
    // 6. CONSTANT_REFERENCE: "<int><op><ref>"
    let re_const_ref = Regex::new(r"^(-?\d+)([-+*/])([A-Z]+[0-9]+)$").unwrap();
    if let Some(caps) = re_const_ref.captures(form) {
        block.reset();
        let val1: i32 = caps.get(1).unwrap().as_str().parse().unwrap();
        let op = caps.get(2).unwrap().as_str().chars().next().unwrap();
        let ref2 = caps.get(3).unwrap().as_str().to_string();
        block.value = Valtype::Int(val1);
        block.data = CellData::CoR { op_code: op, value2: Valtype::Int(val1), cell2: ref2 };
        return;
    }
    // 7. REFERENCE_CONSTANT: "<ref><op><int>"
    let re_ref_const = Regex::new(r"^([A-Z]+[0-9]+)([-+*/])(-?\d+)$").unwrap();
    if let Some(caps) = re_ref_const.captures(form) {
        block.reset();
        let ref1 = caps.get(1).unwrap().as_str().to_string();
        let op = caps.get(2).unwrap().as_str().chars().next().unwrap();
        let val1: i32 = caps.get(3).unwrap().as_str().parse().unwrap();
        block.data = CellData::RoC { op_code: op, value2: Valtype::Int(val1), cell1: ref1 };
        return;
    }
    // 8. REFERENCE_REFERENCE: "<ref><op><ref>"
    let re_ref_ref = Regex::new(r"^([A-Z]+[0-9]+)([-+*/])([A-Z]+[0-9]+)$").unwrap();
    if let Some(caps) = re_ref_ref.captures(form) {
        block.reset();
        let ref1 = caps.get(1).unwrap().as_str().to_string();
        let op = caps.get(2).unwrap().as_str().chars().next().unwrap();
        let ref2 = caps.get(3).unwrap().as_str().to_string();
        block.data = CellData::RoR { op_code: op, cell1: ref1, cell2: ref2 };
        return;
    }
    // 9. RANGE_FUNCTION: "<func>(<ref1>:<ref2>)"
    let re_range_func = Regex::new(r"^([A-Z]+)\(([A-Z]+[0-9]+):([A-Z]+[0-9]+)\)$").unwrap();
    if let Some(caps) = re_range_func.captures(form) {
        block.reset();
        let func = caps.get(1).unwrap().as_str().to_string();
        let ref1 = caps.get(2).unwrap().as_str().to_string();
        let ref2 = caps.get(3).unwrap().as_str().to_string();
        block.data = CellData::Range { cell1: ref1, cell2: ref2, value2: Valtype::Str(func) };
        return;
    }
    block.data = CellData::Invalid;
}

pub fn eval(
    sheet: &Vec<Vec<Cell>>,
    total_rows: usize,
    total_cols: usize,
    r: usize,
    c: usize,
) -> Valtype {
    unsafe {
        EVAL_ERROR = false;
        STATUS_CODE = 0;
    }
    let err_value = Valtype::Str("ERR".to_string());
    let get_cell_val = |reference: &String| -> Option<i32> {
        let (r_idx, c_idx) = to_indices(reference);
        if r_idx < total_rows && c_idx < total_cols {
            let cell = &sheet[r_idx][c_idx];
            match &cell.value {
                Valtype::Int(val) => Some(*val),
                Valtype::Str(_) => {
                    unsafe {
                        EVAL_ERROR = true;
                    }
                    None
                }
            }
        } else {
            unsafe {
                STATUS_CODE = 1;
            }
            None
        }
    };
    let parsed = sheet[r][c].clone();
    let result: i32 = match parsed.data {
        CellData::Const => match parsed.value {
            Valtype::Int(val) => val,
            Valtype::Str(_) => {
                unsafe {
                    EVAL_ERROR = true;
                }
                0
            }
        },
        CellData::Ref { ref cell1 } => {
            get_cell_val(cell1).unwrap_or(0)
        }
        CellData::CoC { op_code, ref value2 } => {
            let v1 = match parsed.value {
                Valtype::Int(val) => val,
                Valtype::Str(_) => {
                    unsafe { EVAL_ERROR = true; }
                    0
                }
            };
            let v2 = match value2 {
                Valtype::Int(val) => *val,
                Valtype::Str(_) => {
                    unsafe { EVAL_ERROR = true; }
                    0
                }
            };
            compute(v1, Some(op_code), v2)
        }
        CellData::CoR { op_code, ref value2, ref cell2 } => {
            let v1 = match value2 {
                Valtype::Int(val) => *val,
                Valtype::Str(_) => {
                    unsafe { EVAL_ERROR = true; }
                    0
                }
            };
            if let Some(v2) = get_cell_val(cell2) {
                compute(v1, Some(op_code), v2)
            } else {
                0
            }
        }
        CellData::RoC { op_code, ref value2, ref cell1 } => {
            let v1 = match value2 {
                Valtype::Int(val) => *val,
                Valtype::Str(_) => {
                    unsafe { EVAL_ERROR = true; }
                    0
                }
            };
            if let Some(v2) = get_cell_val(cell1) {
                compute(v2, Some(op_code), v1)
            } else {
                0
            }
        }
        CellData::RoR { op_code, ref cell1, ref cell2 } => {
            let v1 = get_cell_val(cell1).unwrap_or(0);
            let v2 = get_cell_val(cell2).unwrap_or(0);
            compute(v1, Some(op_code), v2)
        }
        CellData::Range { ref cell1, ref cell2, ref value2 } => {
            if let Valtype::Str(func) = value2 {
                let (r1, c1) = to_indices(cell1);
                let (r2, c2) = to_indices(cell2);
                if r1 < total_rows && c1 < total_cols && r2 < total_rows && c2 < total_cols && r1 <= r2 && c1 <= c2 {
                    let choice = match func.to_uppercase().as_str() {
                        "MAX" => 1,
                        "MIN" => 2,
                        "AVG" => 3,
                        "SUM" => 4,
                        "STDEV" => 5,
                        _ => {
                            unsafe { STATUS_CODE = 2; }
                            0
                        }
                    };
                    compute_range(sheet, r1, r2, c1, c2, choice)
                } else {
                    unsafe { STATUS_CODE = 1; }
                    0
                }
            } else {
                0
            }
        }
        CellData::SleepC => match parsed.value {
            Valtype::Int(val) => {
                sleepy(val);
                val
            }
            Valtype::Str(_) => 0,
        },
        CellData::SleepR { ref cell1 } => {
            if let Some(val) = get_cell_val(cell1) {
                sleepy(val);
                val
            } else {
                0
            }
        }
        CellData::Invalid => {
            unsafe { STATUS_CODE = 2; }
            0
        }
        _ => 0,
    };
    if unsafe { EVAL_ERROR } {
        err_value
    } else {
        Valtype::Int(result)
    }
}

pub fn recalc(
    sheet: &mut Vec<Vec<Cell>>,
    total_rows: usize,
    total_cols: usize,
    start_row: usize,
    start_col: usize,
) {
    type Coord = (usize, usize);
    let mut affected: Vec<Coord> = Vec::with_capacity(50);
    let mut index_map: HashMap<usize, usize> = HashMap::with_capacity(20);
    let mut queue: VecDeque<Coord> = VecDeque::with_capacity(50);

    // BFS to find affected cells
    let key = start_row * total_cols + start_col;
    index_map.insert(key, 0);
    affected.push((start_row, start_col));
    queue.push_back((start_row, start_col));

    while let Some((r, c)) = queue.pop_front() {
        for &(dep_r, dep_c) in &sheet[r][c].dependents {
            if index_map.contains_key(&(dep_r * total_cols + dep_c)) {
                continue;
            }
            let idx = affected.len();
            index_map.insert(dep_r * total_cols + dep_c, idx);
            affected.push((dep_r, dep_c));
            queue.push_back((dep_r, dep_c));
        }
    }

    let n = affected.len();
    let mut in_degree = vec![0; n];
    for &(r, c) in &affected {
        for &(dep_r, dep_c) in &sheet[r][c].dependents {
            let key = dep_r * total_cols + dep_c;
            if let Some(&idx) = index_map.get(&key) {
                in_degree[idx] += 1;
            }
        }
    }
    if in_degree[0] > 0 {
        unsafe { STATUS_CODE = 3; }
        return;
    }
    
    let mut zero_queue: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut i = 0;
    let mut temp_values: HashMap<Coord, Valtype> = HashMap::new();
    while i < zero_queue.len() {
        let idx = zero_queue[i];
        i += 1;
        let (r, c) = affected[idx];
        // Only recalc if the cell contains a formula (i.e. is not Empty)
        match sheet[r][c].data {
            CellData::Empty => {},
            _ => {
                let new_value = eval(&sheet, total_rows, total_cols, r, c);
                temp_values.insert((r, c), sheet[r][c].value.clone());
                sheet[r][c].value = new_value.clone();
            }
        }
        for &(dep_r, dep_c) in &sheet[r][c].dependents {
            let key = dep_r * total_cols + dep_c;
            if let Some(&dep_idx) = index_map.get(&key) {
                in_degree[dep_idx] -= 1;
                if in_degree[dep_idx] == 0 {
                    zero_queue.push(dep_idx);
                }
            }
        }
    }
}
