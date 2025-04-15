use crate::{Cell, CellData, STATUS_CODE, Valtype, CellName};
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
            let cell_ref = CellName::new(m.as_str()).unwrap();
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
            let cell_ref = CellName::new(m.as_str()).unwrap();
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
        let ref2 = CellName::new(caps.get(3).unwrap().as_str()).unwrap();
        block.value = Valtype::Int(val1);
        block.data = CellData::CoR { op_code: op, value2: Valtype::Int(val1), cell2: ref2 };
        return;
    }
    // 7. REFERENCE_CONSTANT: "<ref><op><int>"
    let re_ref_const = Regex::new(r"^([A-Z]+[0-9]+)([-+*/])(-?\d+)$").unwrap();
    if let Some(caps) = re_ref_const.captures(form) {
        block.reset();
        let ref1 = CellName::new(caps.get(1).unwrap().as_str()).unwrap();
        let op = caps.get(2).unwrap().as_str().chars().next().unwrap();
        let val1: i32 = caps.get(3).unwrap().as_str().parse().unwrap();
        block.data = CellData::RoC { op_code: op, value2: Valtype::Int(val1), cell1: ref1 };
        return;
    }
    // 8. REFERENCE_REFERENCE: "<ref><op><ref>"
    let re_ref_ref = Regex::new(r"^([A-Z]+[0-9]+)([-+*/])([A-Z]+[0-9]+)$").unwrap();
    if let Some(caps) = re_ref_ref.captures(form) {
        block.reset();
        let ref1 = CellName::new(caps.get(1).unwrap().as_str()).unwrap();
        let op = caps.get(2).unwrap().as_str().chars().next().unwrap();
        let ref2 = CellName::new(caps.get(3).unwrap().as_str()).unwrap();
        block.data = CellData::RoR { op_code: op, cell1: ref1, cell2: ref2 };
        return;
    }
    // 9. RANGE_FUNCTION: "<func>(<ref1>:<ref2>)"
    let re_range_func = Regex::new(r"^([A-Z]+)\(([A-Z]+[0-9]+):([A-Z]+[0-9]+)\)$").unwrap();
    if let Some(caps) = re_range_func.captures(form) {
        block.reset();
        let func = caps.get(1).unwrap().as_str();
        let ref1 = CellName::new(caps.get(2).unwrap().as_str()).unwrap();
        let ref2 = CellName::new(caps.get(3).unwrap().as_str()).unwrap();
        // Wrap the function name as a CellName
        block.data = CellData::Range { cell1: ref1, cell2: ref2, value2: Valtype::Str(CellName::new(func).unwrap()) };
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
    // Error value now uses a compact cell name
    let err_value = Valtype::Str(CellName::new("ERR").unwrap());
    let get_cell_val = |reference: &CellName| -> Option<i32> {
        let (r_idx, c_idx) = to_indices(reference.as_str());
        if r_idx < total_rows && c_idx < total_cols {
            let cell = &sheet[r_idx][c_idx];
            match &cell.value {
                Valtype::Int(val) => Some(*val),
                Valtype::Str(_) => {
                    unsafe { EVAL_ERROR = true; }
                    None
                }
            }
        } else {
            unsafe { STATUS_CODE = 1; }
            None
        }
    };
    let parsed = sheet[r][c].clone();
    let result: i32 = match parsed.data {
        CellData::Const => match parsed.value {
            Valtype::Int(val) => val,
            Valtype::Str(_) => {
                unsafe { EVAL_ERROR = true; }
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
                // Now when matching a string cell name, error out
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
                let (r1, c1) = to_indices(cell1.as_str());
                let (r2, c2) = to_indices(cell2.as_str());
                if r1 < total_rows && c1 < total_cols && r2 < total_rows && c2 < total_cols && r1 <= r2 && c1 <= c2 {
                    let choice = match func.as_str().to_uppercase().as_str() {
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

pub fn update_and_recalc(
    sheet: &mut Vec<Vec<Cell>>,
    total_rows: usize,
    total_cols: usize,
    r: usize,
    c: usize,
    backup: Cell,
) {
    type Coord = (usize, usize);
    // Validation block from update_cell
    {
        match &sheet[r][c].data {
            CellData::Invalid => {
                unsafe { STATUS_CODE = 2; }
                return;
            }
            CellData::Range { cell1, cell2, .. } => {
                let (row_idx, col_idx) = to_indices(cell1.as_str());
                if row_idx >= total_rows || col_idx >= total_cols {
                    unsafe { STATUS_CODE = 1; }
                    return;
                }
                let (row_idx2, col_idx2) = to_indices(cell2.as_str());
                if row_idx2 >= total_rows || col_idx2 >= total_cols {
                    unsafe { STATUS_CODE = 1; }
                    return;
                }
            }
            CellData::Ref { cell1 } => {
                let (row_idx, col_idx) = to_indices(cell1.as_str());
                if row_idx >= total_rows || col_idx >= total_cols {
                    unsafe { STATUS_CODE = 1; }
                    return;
                }
            }
            CellData::CoR { cell2, .. } => {
                let (row_idx, col_idx) = to_indices(cell2.as_str());
                if row_idx >= total_rows || col_idx >= total_cols {
                    unsafe { STATUS_CODE = 1; }
                    return;
                }
            }
            CellData::RoC { cell1, .. } => {
                let (row_idx, col_idx) = to_indices(cell1.as_str());
                if row_idx >= total_rows || col_idx >= total_cols {
                    unsafe { STATUS_CODE = 1; }
                    return;
                }
            }
            CellData::RoR { cell1, cell2, .. } => {
                let (row_idx, col_idx) = to_indices(cell1.as_str());
                if row_idx >= total_rows || col_idx >= total_cols {
                    unsafe { STATUS_CODE = 1; }
                    return;
                }
                let (row_idx2, col_idx2) = to_indices(cell2.as_str());
                if row_idx2 >= total_rows || col_idx2 >= total_cols {
                    unsafe { STATUS_CODE = 1; }
                    return;
                }
            }
            CellData::SleepR { cell1 } => {
                let (row_idx, col_idx) = to_indices(cell1.as_str());
                if row_idx >= total_rows || col_idx >= total_cols {
                    unsafe { STATUS_CODE = 1; }
                    return;
                }
            }
            _ => {}
        }
    }
    if unsafe { STATUS_CODE } != 0 {
        return;
    }

    // Calculate a single key from (r, c)
    let cell_hash = (r * total_cols + c) as u32;

    // Store original value for rollback
    let original_value = sheet[r][c].clone();

    // Remove old dependencies from backup
    match backup.data {
        CellData::Range { ref cell1, ref cell2, .. } => {
            let (start_row, start_col) = to_indices(cell1.as_str());
            let (end_row, end_col) = to_indices(cell2.as_str());
            for i in start_row..=end_row {
                for j in start_col..=end_col {
                    if i < total_rows && j < total_cols {
                        sheet[i][j].dependents.remove(&cell_hash);
                    }
                }
            }
        }
        CellData::Ref { ref cell1 } => {
            let (i, j) = to_indices(cell1.as_str());
            if i < total_rows && j < total_cols {
                sheet[i][j].dependents.remove(&cell_hash);
            }
        }
        CellData::CoR { ref cell2, .. } => {
            let (i, j) = to_indices(cell2.as_str());
            if i < total_rows && j < total_cols {
                sheet[i][j].dependents.remove(&cell_hash);
            }
        }
        CellData::RoC { ref cell1, .. } => {
            let (i, j) = to_indices(cell1.as_str());
            if i < total_rows && j < total_cols {
                sheet[i][j].dependents.remove(&cell_hash);
            }
        }
        CellData::RoR { ref cell1, ref cell2, .. } => {
            let (i, j) = to_indices(cell1.as_str());
            if i < total_rows && j < total_cols {
                sheet[i][j].dependents.remove(&cell_hash);
            }
            let (i2, j2) = to_indices(cell2.as_str());
            if i2 < total_rows && j2 < total_cols {
                sheet[i2][j2].dependents.remove(&cell_hash);
            }
        }
        CellData::SleepR { ref cell1 } => {
            let (i, j) = to_indices(cell1.as_str());
            if i < total_rows && j < total_cols {
                sheet[i][j].dependents.remove(&cell_hash);
            }
        }
        _ => {}
    }

    // Add new dependencies from current data
    let current_data = sheet[r][c].data.clone();
    match current_data {
        CellData::Range { cell1, cell2, .. } => {
            let (start_row, start_col) = to_indices(cell1.as_str());
            let (end_row, end_col) = to_indices(cell2.as_str());
            for row in start_row..=end_row {
                for col in start_col..=end_col {
                    if row < total_rows && col < total_cols {
                        sheet[row][col].dependents.insert(cell_hash);
                    }
                }
            }
        }
        CellData::Ref { cell1 } => {
            let (dep_row, dep_col) = to_indices(cell1.as_str());
            if dep_row < total_rows && dep_col < total_cols {
                sheet[dep_row][dep_col].dependents.insert(cell_hash);
            }
        }
        CellData::CoR { cell2, .. } => {
            let (dep_row, dep_col) = to_indices(cell2.as_str());
            if dep_row < total_rows && dep_col < total_cols {
                sheet[dep_row][dep_col].dependents.insert(cell_hash);
            }
        }
        CellData::RoC { cell1, .. } => {
            let (dep_row, dep_col) = to_indices(cell1.as_str());
            if dep_row < total_rows && dep_col < total_cols {
                sheet[dep_row][dep_col].dependents.insert(cell_hash);
            }
        }
        CellData::RoR { cell1, cell2, .. } => {
            let (dep_row, dep_col) = to_indices(cell1.as_str());
            if dep_row < total_rows && dep_col < total_cols {
                sheet[dep_row][dep_col].dependents.insert(cell_hash);
            }
            let (dep_row2, dep_col2) = to_indices(cell2.as_str());
            if dep_row2 < total_rows && dep_col2 < total_cols {
                sheet[dep_row2][dep_col2].dependents.insert(cell_hash);
            }
        }
        CellData::SleepR { cell1 } => {
            let (dep_row, dep_col) = to_indices(cell1.as_str());
            if dep_row < total_rows && dep_col < total_cols {
                sheet[dep_row][dep_col].dependents.insert(cell_hash);
            }
        }
        _ => {}
    }

    // Recalculation block from recalc
    let mut affected: Vec<Coord> = Vec::with_capacity(50);
    let mut index_map: HashMap<usize, usize> = HashMap::with_capacity(20);
    let mut queue: VecDeque<Coord> = VecDeque::with_capacity(50);

    // BFS to find affected cells
    let key = r * total_cols + c;
    index_map.insert(key, 0);
    affected.push((r, c));
    queue.push_back((r, c));

    while let Some((r_curr, c_curr)) = queue.pop_front() {
        // Iterate over dependent keys (u32 values)
        for &dep_key in &sheet[r_curr][c_curr].dependents {
            // Convert dep_key to (dep_r, dep_c)
            let dep_r = (dep_key as usize) / total_cols;
            let dep_c = (dep_key as usize) % total_cols;
            let computed_key = dep_r * total_cols + dep_c;
            if index_map.contains_key(&computed_key) {
                continue;
            }
            let idx = affected.len();
            index_map.insert(computed_key, idx);
            affected.push((dep_r, dep_c));
            queue.push_back((dep_r, dep_c));
        }
    }

    let n = affected.len();
    let mut in_degree = vec![0; n];
    for &(r_curr, c_curr) in &affected {
        for &dep_key in &sheet[r_curr][c_curr].dependents {
            let key = {
                let dep_r = (dep_key as usize) / total_cols;
                let dep_c = (dep_key as usize) % total_cols;
                dep_r * total_cols + dep_c
            };
            if let Some(&idx) = index_map.get(&key) {
                in_degree[idx] += 1;
            }
        }
    }
    if in_degree[0] >0 {
        sheet[r][c] =backup;
        unsafe { STATUS_CODE = 3; }
        return;
    }
    let mut zero_queue: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut i = 0;
    let mut temp_values: HashMap<Coord, Valtype> = HashMap::new();

    while i < zero_queue.len() {
        let idx = zero_queue[i];
        i += 1;
        let (r_curr, c_curr) = affected[idx];
        match sheet[r_curr][c_curr].data {
            CellData::Empty => {}
            _ => {
                let new_value = eval(&sheet, total_rows, total_cols, r_curr, c_curr);
                temp_values.insert((r_curr, c_curr), sheet[r_curr][c_curr].value.clone());
                sheet[r_curr][c_curr].value = new_value;
            }
        }
        for &dep_key in &sheet[r_curr][c_curr].dependents {
            let key = {
                let dep_r = (dep_key as usize) / total_cols;
                let dep_c = (dep_key as usize) % total_cols;
                dep_r * total_cols + dep_c
            };
            if let Some(&dep_idx) = index_map.get(&key) {
                in_degree[dep_idx] -= 1;
                if in_degree[dep_idx] == 0 {
                    zero_queue.push(dep_idx);
                }
            }
        }
    }

}
