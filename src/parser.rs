use crate::utils::{to_indices, compute, sleepy, compute_range, EVAL_ERROR, STATUS_CODE};
use crate::Cell;
use crate::CellValue;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, PartialEq)]
pub enum FormulaType {
    SleepConst,
    SleepRef,
    Constant,
    Reference,
    ConstantConstant,
    ConstantReference,
    ReferenceConstant,
    ReferenceReference,
    RangeFunction,
    InvalidFormula,
}

impl Default for FormulaType {
    fn default() -> Self {
        FormulaType::InvalidFormula
    }
}

#[derive(Debug, Default)]
pub struct ParsedFormula {
    pub formula_type: FormulaType,
    pub ref1: Option<String>,
    pub ref2: Option<String>,
    pub op: Option<char>,
    pub func: Option<String>,
    pub val1: Option<i32>,
    pub val2: Option<i32>,
}

use regex::Regex;

pub fn detect_formula(form: &str) -> ParsedFormula {
    let form = form.trim();

    // 1. SLEEP_CONST: "SLEEP(<int>)"
    let re_sleep_const = Regex::new(r"^SLEEP\((\d+)\)$").unwrap();
    if let Some(caps) = re_sleep_const.captures(form) {
        if let Some(m) = caps.get(1) {
            if let Ok(val) = m.as_str().parse::<i32>() {
                return ParsedFormula {
                    formula_type: FormulaType::SleepConst,
                    val1: Some(val),
                    ..Default::default()
                };
            }
        }
    }   

    // 2. SLEEP_REF: "SLEEP(<ref>)" where <ref> is e.g. A1
    let re_sleep_ref = Regex::new(r"^SLEEP\(([A-Z]+[0-9]+)\)$").unwrap();
    if let Some(caps) = re_sleep_ref.captures(form) {
        if let Some(m) = caps.get(1) {
            let cell_ref = m.as_str().to_string();
            return ParsedFormula {
                formula_type: FormulaType::SleepRef,
                ref1: Some(cell_ref),
                ..Default::default()
            };
        }
    }

    // 3. CONSTANT: a lone integer
    let re_constant = Regex::new(r"^(\d+)$").unwrap();
    if let Some(caps) = re_constant.captures(form) {
        if let Some(m) = caps.get(1) {
            if let Ok(val) = m.as_str().parse::<i32>() {
                return ParsedFormula {
                    formula_type: FormulaType::Constant,
                    val1: Some(val),
                    ..Default::default()
                };
            }
        }
    }

    // 4. REFERENCE: a cell reference (e.g., "A1")
    let re_reference = Regex::new(r"^([A-Z]+[0-9]+)$").unwrap();
    if let Some(caps) = re_reference.captures(form) {
        if let Some(m) = caps.get(1) {
            return ParsedFormula {
                formula_type: FormulaType::Reference,
                ref1: Some(m.as_str().to_string()),
                ..Default::default()
            };
        }
    }

    // 5. CONSTANT_CONSTANT: "<int><op><int>"
    let re_const_const = Regex::new(r"^(\d+)([-+*/])(\d+)$").unwrap();
    if let Some(caps) = re_const_const.captures(form) {
        let val1: i32 = caps.get(1).unwrap().as_str().parse().unwrap();
        let op = caps.get(2).unwrap().as_str().chars().next().unwrap();
        let val2: i32 = caps.get(3).unwrap().as_str().parse().unwrap();
        return ParsedFormula {
            formula_type: FormulaType::ConstantConstant,
            val1: Some(val1),
            val2: Some(val2),
            op: Some(op),
            ..Default::default()
        };
    }

    // 6. CONSTANT_REFERENCE: "<int><op><ref>"
    let re_const_ref = Regex::new(r"^(\d+)([-+*/])([A-Z]+[0-9]+)$").unwrap();
    if let Some(caps) = re_const_ref.captures(form) {
        let val1: i32 = caps.get(1).unwrap().as_str().parse().unwrap();
        let op = caps.get(2).unwrap().as_str().chars().next().unwrap();
        let ref2 = caps.get(3).unwrap().as_str().to_string();
        return ParsedFormula {
            formula_type: FormulaType::ConstantReference,
            val1: Some(val1),
            ref2: Some(ref2),
            op: Some(op),
            ..Default::default()
        };
    }

    // 7. REFERENCE_CONSTANT: "<ref><op><int>"
    let re_ref_const = Regex::new(r"^([A-Z]+[0-9]+)([-+*/])(\d+)$").unwrap();
    if let Some(caps) = re_ref_const.captures(form) {
        let ref1 = caps.get(1).unwrap().as_str().to_string();
        let op = caps.get(2).unwrap().as_str().chars().next().unwrap();
        let val1: i32 = caps.get(3).unwrap().as_str().parse().unwrap();
        return ParsedFormula {
            formula_type: FormulaType::ReferenceConstant,
            ref1: Some(ref1),
            val1: Some(val1),
            op: Some(op),
            ..Default::default()
        };
    }

    // 8. REFERENCE_REFERENCE: "<ref><op><ref>"
    let re_ref_ref = Regex::new(r"^([A-Z]+[0-9]+)([-+*/])([A-Z]+[0-9]+)$").unwrap();
    if let Some(caps) = re_ref_ref.captures(form) {
        let ref1 = caps.get(1).unwrap().as_str().to_string();
        let op = caps.get(2).unwrap().as_str().chars().next().unwrap();
        let ref2 = caps.get(3).unwrap().as_str().to_string();
        return ParsedFormula {
            formula_type: FormulaType::ReferenceReference,
            ref1: Some(ref1),
            ref2: Some(ref2),
            op: Some(op),
            ..Default::default()
        };
    }

    // 9. RANGE_FUNCTION: "<func>(<ref1>:<ref2>)"
    let re_range_func = Regex::new(r"^([A-Z]+)\(([A-Z]+[0-9]+):([A-Z]+[0-9]+)\)$").unwrap();
    if let Some(caps) = re_range_func.captures(form) {
        let func = caps.get(1).unwrap().as_str().to_string();
        let ref1 = caps.get(2).unwrap().as_str().to_string();
        let ref2 = caps.get(3).unwrap().as_str().to_string();
        return ParsedFormula {
            formula_type: FormulaType::RangeFunction,
            func: Some(func),
            ref1: Some(ref1),
            ref2: Some(ref2),
            ..Default::default()
        };
    }

    // If none of the patterns matched, return an invalid formula.
    ParsedFormula {
        formula_type: FormulaType::InvalidFormula,
        ..Default::default()
    }
}

pub fn eval(sheet: &Vec<Vec<Cell>>, total_rows: usize, total_cols: usize, form: &str) -> CellValue {
    unsafe {
        EVAL_ERROR = false;
        STATUS_CODE = 0;
    }
    let err_value = CellValue::Str("error".to_string());
    let parsed = detect_formula(form);
    
    let get_cell_val = |reference: &String| -> Option<i32> {
        let (r, c) = to_indices(reference);
        if r >= total_rows || c >= total_cols || unsafe { STATUS_CODE } == 1{
            unsafe { STATUS_CODE = 1; }
            None
        } else {
            match &sheet[r][c].value {
                CellValue::Int(val) => Some(*val),
                CellValue::Str(_) => {
                    unsafe { EVAL_ERROR = true; }
                    None
                }
            }
        }
    };

    let result = match parsed.formula_type {
        FormulaType::Constant => parsed.val1.unwrap_or(0),
        FormulaType::Reference => {
            if let Some(ref reference) = parsed.ref1 {
                get_cell_val(reference).unwrap_or(0)
            } else {
                0
            }
        }
        FormulaType::ConstantConstant => {
            let v1 = parsed.val1.unwrap_or(0);
            let v2 = parsed.val2.unwrap_or(0);
            compute(v1, parsed.op, v2)
        }
        FormulaType::ConstantReference => {
            let v1 = parsed.val1.unwrap_or(0);
            if let Some(ref reference) = parsed.ref2 {
                if let Some(v2) = get_cell_val(reference) {
                    compute(v1, parsed.op, v2)
                } else {
                    0
                }
            } else {
                0
            }
        }
        FormulaType::ReferenceConstant => {
            if let Some(ref reference) = parsed.ref1 {
                if let Some(v) = get_cell_val(reference) {
                    compute(v, parsed.op, parsed.val1.unwrap_or(0))
                } else {
                    0
                }
            } else {
                0
            }
        }
        FormulaType::ReferenceReference => {
            if let (Some(ref r1_str), Some(ref r2_str)) = (parsed.ref1.as_ref(), parsed.ref2.as_ref()) {
                let v1 = get_cell_val(r1_str).unwrap_or(0);
                let v2 = get_cell_val(r2_str).unwrap_or(0);
                compute(v1, parsed.op, v2)
            } else {
                0
            }
        }
        FormulaType::RangeFunction => {
            if let (Some(ref func), Some(ref r1_str), Some(ref r2_str)) =
                (parsed.func.as_ref(), parsed.ref1.as_ref(), parsed.ref2.as_ref())
            {
                let (r1, c1) = to_indices(r1_str);
                let (r2, c2) = to_indices(r2_str);
                if r1 >= total_rows || c1 >= total_cols || r2 >= total_rows || c2 >= total_cols {
                    unsafe { STATUS_CODE = 1; }
                    0
                } else {
                    let choice = match func.to_uppercase().as_str() {
                        "MAX" => 1,
                        "MIN" => 2,
                        "AVG" => 3,
                        "SUM" => 4,
                        "STDEV" => 5,
                        _ => {
                            unsafe { STATUS_CODE = 2; }
                            return err_value;
                        }
                    };
                    let res = compute_range(
                        sheet,
                        r1.min(r2),
                        r1.max(r2),
                        c1.min(c2),
                        c1.max(c2),
                        choice,
                    );
                    res
                }
            } else {
                0
            }
        }
        FormulaType::SleepConst => {
            let val = parsed.val1.unwrap_or(0);
            sleepy(val);
            val
        }
        FormulaType::SleepRef => {
            if let Some(ref reference) = parsed.ref1 {
                if let Some(val) = get_cell_val(reference) {
                    sleepy(val);
                    val
                } else {
                    0
                }
            } else {
                0
            }
        }
        FormulaType::InvalidFormula => {
            unsafe { STATUS_CODE = 2; }
            0
        }
    };
    
    if unsafe { EVAL_ERROR }{
        err_value
    } else {
        CellValue::Int(result)
    }
}


type Coord = (usize, usize);

pub fn recalc(sheet: &mut Vec<Vec<Cell>>, total_rows: usize, total_cols: usize, start_row: usize, start_col: usize) {
    let mut affected: Vec<Coord> = Vec::with_capacity(50);
    let mut vis: HashMap<usize, usize> = HashMap::with_capacity(20);
    let mut bfs: VecDeque<Coord> = VecDeque::with_capacity(50);

    let key = start_row * total_cols + start_col;
    vis.insert(key, 0);
    affected.push((start_row, start_col));
    bfs.push_back((start_row, start_col));

    while let Some((r, c)) = bfs.pop_front() {
        for &(dep_r_u8, dep_c_u8) in &sheet[r][c].dependents {
            let dep_r = dep_r_u8 as usize;
            let dep_c = dep_c_u8 as usize;
            let key = dep_r * total_cols + dep_c;
            if vis.contains_key(&key) {
                continue;
            }
            let idx = affected.len();
            vis.insert(key, idx);
            affected.push((dep_r, dep_c));
            bfs.push_back((dep_r, dep_c));
        }
    }

    let affected_count = affected.len();
    let mut in_degree = vec![0; affected_count];
    for &(r, c) in &affected {
        for &(dep_r_u8, dep_c_u8) in &sheet[r][c].dependents {
            let key = (dep_r_u8 as usize) * total_cols + (dep_c_u8 as usize);
            if let Some(&dep_idx) = vis.get(&key) {
                in_degree[dep_idx] += 1;
            }
        }
    }

    let mut zero_queue: Vec<usize> = (0..affected_count)
        .filter(|&i| in_degree[i] == 0)
        .collect();

    let mut i = 0;
    while i < zero_queue.len() {
        let idx = zero_queue[i];
        i += 1;
        let (r, c) = affected[idx];
        if let Some(ref formula) = sheet[r][c].formula {
            sheet[r][c].value = eval(&sheet, total_rows, total_cols, formula);
        }
        for &(dep_r_u8, dep_c_u8) in &sheet[r][c].dependents {
            let key = (dep_r_u8 as usize) * total_cols + (dep_c_u8 as usize);
            if let Some(&dep_idx) = vis.get(&key) {
                in_degree[dep_idx] -= 1;
                if in_degree[dep_idx] == 0 {
                    zero_queue.push(dep_idx);
                }
            }
        }
    }
}
