use std::collections::{
    HashMap,
    VecDeque,HashSet
};
use regex::Regex;

use crate::utils::*;
use crate::{
    Cell,
    CellData,
    CellName,
    STATUS_CODE,
    Valtype,
};

pub fn detect_formula(
    block: &mut Cell,
    form: &str,
) {
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
    sheet: &HashMap<u32, Cell>,
    total_rows: usize,
    total_cols: usize,
    r: usize,
    c: usize,
) -> Valtype {
    unsafe {
        EVAL_ERROR = false;
        STATUS_CODE = 0;
    }
    let err_value = Valtype::Str(CellName::new("ERR").unwrap());
 
    // lookup-or-default
    let key = (r * total_cols + c) as u32;
    let parsed = sheet.get(&key).cloned().unwrap_or(Cell {
        value: Valtype::Int(0),
        data:  CellData::Empty,
        dependents: Default::default(),
    });
 
    // helper for single‑cell refs
    let get_cell_val = |ref_name: &CellName| -> Option<i32> {
        let (ri, ci) = to_indices(ref_name.as_str());
        if ri < total_rows && ci < total_cols {
            let idx = (ri * total_cols + ci) as u32;
            match sheet.get(&idx).map(|c| &c.value).unwrap_or(&Valtype::Int(0)) {
                Valtype::Int(v) => Some(*v),
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
 
    let result: i32 = match parsed.data {
        CellData::Const => match parsed.value {
            Valtype::Int(v) => v,
            Valtype::Str(_) => { unsafe { EVAL_ERROR = true; } 0 }
        },
        CellData::Ref { ref cell1 } => get_cell_val(cell1).unwrap_or(0),
        CellData::CoC { op_code, ref value2 } => {
            let v1 = if let Valtype::Int(v) = parsed.value { v } else { unsafe { EVAL_ERROR = true; } 0 };
            let v2 = if let Valtype::Int(v) = *value2      { v } else { unsafe { EVAL_ERROR = true; } 0 };
            compute(v1, Some(op_code), v2)
        }
        CellData::CoR { op_code, ref value2, ref cell2 } => {
            let v1 = if let Valtype::Int(v) = *value2 { v } else { unsafe { EVAL_ERROR = true; } 0 };
            if let Some(v2) = get_cell_val(cell2) { compute(v1, Some(op_code), v2) } else { 0 }
        }
        CellData::RoC { op_code, ref value2, ref cell1 } => {
            let v2 = if let Valtype::Int(v) = *value2 { v } else { unsafe { EVAL_ERROR = true; } 0 };
            if let Some(v1) = get_cell_val(cell1) { compute(v1, Some(op_code), v2) } else { 0 }
        }
        CellData::RoR { op_code, ref cell1, ref cell2 } => {
            let v1 = get_cell_val(cell1).unwrap_or(0);
            let v2 = get_cell_val(cell2).unwrap_or(0);
            compute(v1, Some(op_code), v2)
        }
        CellData::Range { cell1, cell2, value2: Valtype::Str(func) } => {
            let (r1, c1) = to_indices(cell1.as_str());
            let (r2, c2) = to_indices(cell2.as_str());
            if r1 <= r2 && c1 <= c2 && r2 < total_rows && c2 < total_cols {
                let choice = match func.as_str().to_uppercase().as_str() {
                    "MAX" => 1, "MIN" => 2, "AVG" => 3, "SUM" => 4, "STDEV" => 5,
                    _       => { unsafe { STATUS_CODE = 2; } 0 }
                };
                compute_range(sheet, total_cols, r1, r2, c1, c2, choice)
            } else {
                unsafe { STATUS_CODE = 1; }
                0
            }
        }
        CellData::SleepC => {
            if let Valtype::Int(v) = parsed.value {
                sleepy(v);
                v
            } else { 0 }
        }
        CellData::SleepR { ref cell1 } => {
            if let Some(v) = get_cell_val(cell1) {
                sleepy(v);
                v
            } else { 0 }
        }
        CellData::Invalid => {
            unsafe { STATUS_CODE = 2; }
            0
        }
        _ => 0,
    };
 
    if unsafe { EVAL_ERROR } { err_value } else { Valtype::Int(result) }
}


pub fn update_and_recalc(
    sheet: &mut HashMap<u32, Cell>,
    ranged: &mut HashMap<u32, Vec<(u32, u32)>>,
    is_r: &mut Vec<bool>,
    total_rows: usize,
    total_cols: usize,
    r: usize,
    c: usize,
    backup: Cell,
) {
    type Coord = (usize, usize);

    // 1) VALIDATION (unchanged)
    {
        let data = &sheet
            .get(&((r * total_cols + c) as u32))
            .map(|cell| &cell.data)
            .unwrap_or(&CellData::Empty);
        match data {
            CellData::Invalid => { unsafe { STATUS_CODE = 2; } return; }
            CellData::Range { cell1, cell2, .. } => {
                for name in &[cell1, cell2] {
                    let (ri, ci) = to_indices(name.as_str());
                    if ri >= total_rows || ci >= total_cols {
                        unsafe { STATUS_CODE = 1; } return;
                    }
                }
            }
            CellData::Ref { cell1 }
            | CellData::SleepR { cell1 }
            | CellData::RoC { cell1, .. } => {
                let (ri, ci) = to_indices(cell1.as_str());
                if ri >= total_rows || ci >= total_cols {
                    unsafe { STATUS_CODE = 1; } return;
                }
            }
            CellData::CoR { cell2, .. } => {
                let (ri, ci) = to_indices(cell2.as_str());
                if ri >= total_rows || ci >= total_cols {
                    unsafe { STATUS_CODE = 1; } return;
                }
            }
            CellData::RoR { cell1, cell2, .. } => {
                for name in &[cell1, cell2] {
                    let (ri, ci) = to_indices(name.as_str());
                    if ri >= total_rows || ci >= total_cols {
                        unsafe { STATUS_CODE = 1; } return;
                    }
                }
            }
            _ => {}
        }
    }
    if unsafe { STATUS_CODE } != 0 { return; }

    let cell_key = (r * total_cols + c) as u32;

    // 2) REMOVE old dependency edges
    macro_rules! remove_dep {
        ($ri:expr, $ci:expr) => {{
            let idx = ($ri * total_cols + $ci) as u32;
            if let Some(dep) = sheet.get_mut(&idx) {
                dep.dependents.remove(&cell_key);
            }
        }};
    }
    match &backup.data {
        CellData::Range { cell1, cell2, .. } => {
            let (sr, sc) = to_indices(cell1.as_str());
            let (er, ec) = to_indices(cell2.as_str());
            // remove old mapping
            ranged.remove(&cell_key);
            // clear each child’s ranged flag only if not in any other range
            for rr in sr..=er {
                for cc in sc..=ec {
                    let idx = (rr * total_cols + cc) as u32;
                    let still_covered = ranged.iter().any(|(_, ranges)|
                        ranges.iter().any(|&(s, e)| in_range(idx, s, e, total_cols))
                    );
                    is_r[idx as usize] = still_covered;
                }
            }
        }
        CellData::Ref { cell1 } => {
            let (ri, ci) = to_indices(cell1.as_str());
            remove_dep!(ri, ci);
        }
        CellData::CoR { cell2, .. } => {
            let (ri, ci) = to_indices(cell2.as_str());
            remove_dep!(ri, ci);
        }
        CellData::RoC { cell1, .. } => {
            let (ri, ci) = to_indices(cell1.as_str());
            remove_dep!(ri, ci);
        }
        CellData::RoR { cell1, cell2, .. } => {
            let (r1, c1) = to_indices(cell1.as_str()); remove_dep!(r1, c1);
            let (r2, c2) = to_indices(cell2.as_str()); remove_dep!(r2, c2);
        }
        CellData::SleepR { cell1 } => {
            let (ri, ci) = to_indices(cell1.as_str());
            remove_dep!(ri, ci);
        }
        _ => {}
    }

    // 3) ADD new edges
    let new_data = sheet.get(&cell_key).map(|c| c.data.clone()).unwrap_or(CellData::Empty);
    match &new_data {
        CellData::Range { cell1, cell2, .. } => {
            let (sr, sc) = to_indices(cell1.as_str());
            let (er, ec) = to_indices(cell2.as_str());
            ranged.entry(cell_key).or_insert_with(Vec::new)
                .push(((sr * total_cols + sc) as u32, (er * total_cols + ec) as u32));
            for rr in sr..=er {
                for cc in sc..=ec {
                    let idx = (rr * total_cols + cc) as u32;
                    is_r[idx as usize]=true;
                }
            }
        }
        CellData::Ref { cell1 } => {
            let (ri, ci) = to_indices(cell1.as_str());
            let idx = (ri * total_cols + ci) as u32;
            sheet.entry(idx).or_insert_with(|| Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() })
                .dependents.insert(cell_key);
        }
        CellData::CoR { cell2, .. } => {
            let (ri, ci) = to_indices(cell2.as_str());
            let idx = (ri * total_cols + ci) as u32;
            sheet.entry(idx).or_insert_with(|| Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() })
                .dependents.insert(cell_key);
        }
        CellData::RoC { cell1, .. } => {
            let (ri, ci) = to_indices(cell1.as_str());
            let idx = (ri * total_cols + ci) as u32;
            sheet.entry(idx).or_insert_with(|| Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() })
                .dependents.insert(cell_key);
        }
        CellData::RoR { cell1, cell2, .. } => {
            for name in &[cell1, cell2] {
                let (ri, ci) = to_indices(name.as_str());
                let idx = (ri * total_cols + ci) as u32;
                sheet.entry(idx).or_insert_with(|| Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() })
                    .dependents.insert(cell_key);
            }
        }
        CellData::SleepR { cell1 } => {
            let (ri, ci) = to_indices(cell1.as_str());
            let idx = (ri * total_cols + ci) as u32;
            sheet.entry(idx).or_insert_with(|| Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() })
                .dependents.insert(cell_key);
        }
        _ => {}
    }

    // 4) BUILD affected-list via BFS
    let mut affected = Vec::<Coord>::new();
    let mut index_map = HashMap::<u32, usize>::new();
    let mut queue = VecDeque::<Coord>::new();

    affected.push((r, c));
    index_map.insert(cell_key, 0);
    queue.push_back((r, c));

    while let Some((rr, cc)) = queue.pop_front() {
        let idx = (rr * total_cols + cc) as u32;
        // direct dependents
        if let Some(cell) = sheet.get(&idx) {
            for &dep_key in &cell.dependents {
                if !index_map.contains_key(&dep_key) {
                    let dr = (dep_key as usize) / total_cols;
                    let dc = (dep_key as usize) % total_cols;
                    let ni = affected.len();
                    index_map.insert(dep_key, ni);
                    affected.push((dr, dc));
                    queue.push_back((dr, dc));
                }
            }
        }
        // range-based dependents without is_r check
        for (&parent, ranges) in ranged.iter() {
            for &(start, end) in ranges.iter() {
                if in_range(idx, start, end, total_cols) && !index_map.contains_key(&parent) {
                    let pr = (parent as usize) / total_cols;
                    let pc = (parent as usize) % total_cols;
                    let ni = affected.len();
                    index_map.insert(parent, ni);
                    affected.push((pr, pc));
                    queue.push_back((pr, pc));
                }
            }
        }
    }

    // 5) TOPOLOGICAL ORDER & EVAL
    let n = affected.len();
    let mut in_degree = vec![0; n];
    for &(rr, cc) in &affected {
        let idx = (rr * total_cols + cc) as u32;
        if let Some(cell) = sheet.get(&idx) {
            for &dep_key in &cell.dependents {
                if let Some(&j) = index_map.get(&dep_key) {
                    in_degree[j] += 1;
                }
            }
        }
        for (&parent, ranges) in ranged.iter() {
            for &(start, end) in ranges.iter() {
                if in_range(idx, start, end, total_cols) {
                    if let Some(&j) = index_map.get(&parent) {
                        in_degree[j] += 1;
                    }
                }
            }
        }
    }

    // Cycle detection
    if in_degree[0] > 0 {
        // Remove newly added dependency edges
        let new_data = sheet
            .get(&cell_key)
            .map(|c| c.data.clone())
            .unwrap_or(CellData::Empty);
        match &new_data {
            CellData::Range { cell1, cell2, .. } => {
                let (sr, sc) = to_indices(cell1.as_str());
                let (er, ec) = to_indices(cell2.as_str());
                for rr in sr..=er {
                    for cc in sc..=ec {
                        let idx = (rr * total_cols + cc) as u32;
                            is_r[idx as usize] = false;
                    }
                }
                ranged.remove(&cell_key);
            }
            CellData::Ref { cell1 } => {
                let (ri, ci) = to_indices(cell1.as_str());
                let idx = (ri * total_cols + ci) as u32;
                if let Some(dep) = sheet.get_mut(&idx) {
                    dep.dependents.remove(&cell_key);
                }
            }
            CellData::CoR { cell2, .. } => {
                let (ri, ci) = to_indices(cell2.as_str());
                let idx = (ri * total_cols + ci) as u32;
                if let Some(dep) = sheet.get_mut(&idx) {
                    dep.dependents.remove(&cell_key);
                }
            }
            CellData::RoC { cell1, .. } => {
                let (ri, ci) = to_indices(cell1.as_str());
                let idx = (ri * total_cols + ci) as u32;
                if let Some(dep) = sheet.get_mut(&idx) {
                    dep.dependents.remove(&cell_key);
                }
            }
            CellData::RoR { cell1, cell2, .. } => {
                for name in &[cell1, cell2] {
                    let (ri, ci) = to_indices(name.as_str());
                    let idx = (ri * total_cols + ci) as u32;
                    if let Some(dep) = sheet.get_mut(&idx) {
                        dep.dependents.remove(&cell_key);
                    }
                }
            }
            CellData::SleepR { cell1 } => {
                let (ri, ci) = to_indices(cell1.as_str());
                let idx = (ri * total_cols + ci) as u32;
                if let Some(dep) = sheet.get_mut(&idx) {
                    dep.dependents.remove(&cell_key);
                }
            }
            _ => {}
        }

        // Roll back the cell
        *sheet.get_mut(&cell_key).unwrap() = backup;
        unsafe { STATUS_CODE = 3; }
        return;
    }

    // 6) Kahn’s algorithm
    let mut zero_q: Vec<usize> = in_degree.iter().enumerate().filter_map(|(i, &d)| if d == 0 { Some(i) } else { None }).collect();
    while let Some(idx0) = zero_q.pop() {
        let (rr, cc) = affected[idx0];
        let key = (rr * total_cols + cc) as u32;
        if let Some(cell) = sheet.get(&key) {
            if cell.data != CellData::Empty {
                let val = eval(sheet, total_rows, total_cols, rr, cc);
                sheet.get_mut(&key).unwrap().value = val;
            }
            for &dep_key in &sheet.get(&key).unwrap().dependents {
                if let Some(&j) = index_map.get(&dep_key) {
                    in_degree[j] -= 1;
                    if in_degree[j] == 0 {
                        zero_q.push(j);
                    }
                }
            }
        }
        // ranged parents
        for (&parent, ranges) in ranged.iter() {
            for &(start, end) in ranges.iter() {
                if in_range(key, start, end, total_cols) {
                    if let Some(&j) = index_map.get(&parent) {
                        in_degree[j] -= 1;
                        if in_degree[j] == 0 {
                            zero_q.push(j);
                        }
                    }
                }
            }
        }
    }
}
