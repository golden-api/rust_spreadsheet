use sscanf::sscanf;
use crate::utils::{to_indices, compute, sleepy, compute_range, EVAL_ERROR, STATUS_CODE};
use crate::Cell;
use crate::CellValue;

#[derive(Debug, PartialEq)]
enum FormulaType {
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

pub fn detect_formula(form: &str) -> ParsedFormula {
    let form = form.trim();
    // 1. SLEEP_CONST: "SLEEP(<int>)"
    if let Ok(val1) = sscanf!(form, "SLEEP({})", i32) {
        return ParsedFormula {
            formula_type: FormulaType::SleepConst,
            val1: Some(val1),
            ..Default::default()
        };
    }
    // 2. SLEEP_REF: "SLEEP(<ref>)"
    if let Ok(ref1) = sscanf!(form, "SLEEP({})", String) {
        return ParsedFormula {
            formula_type: FormulaType::SleepRef,
            ref1: Some(ref1),
            ..Default::default()
        };
    }
    // 3. CONSTANT: a lone integer
    if let Ok(val1) = sscanf!(form, "{}", i32) {
        return ParsedFormula {
            formula_type: FormulaType::Constant,
            val1: Some(val1),
            ..Default::default()
        };
    }
    // 4. REFERENCE: a cell reference (e.g., "A1")
    if let Ok(ref1) = sscanf!(form, "{}", String) {
        if ref1.chars().all(|c| c.is_ascii_alphanumeric()) {
            return ParsedFormula {
                formula_type: FormulaType::Reference,
                ref1: Some(ref1),
                ..Default::default()
            };
        }
    }
    // 5. CONSTANT_CONSTANT: "<int> <op> <int>"
    if let Ok((val1, op, val2)) = sscanf!(form, "{}{}{}", i32, char, i32) {
        return ParsedFormula {
            formula_type: FormulaType::ConstantConstant,
            val1: Some(val1),
            val2: Some(val2),
            op: Some(op),
            ..Default::default()
        };
    }
    // 6. CONSTANT_REFERENCE: "<int> <op> <ref>"
    if let Ok((val1, op, ref2)) = sscanf!(form, "{}{}{}", i32, char, String) {
        if ref2.chars().all(|c| c.is_ascii_alphanumeric()) {
            return ParsedFormula {
                formula_type: FormulaType::ConstantReference,
                val1: Some(val1),
                ref2: Some(ref2),
                op: Some(op),
                ..Default::default()
            };
        }
    }
    // 7. REFERENCE_CONSTANT: "<ref> <op> <int>"
    if let Ok((ref1, op, val1)) = sscanf!(form, "{}{}{}", String, char, i32) {
        if ref1.chars().all(|c| c.is_ascii_alphanumeric()) {
            return ParsedFormula {
                formula_type: FormulaType::ReferenceConstant,
                ref1: Some(ref1),
                val1: Some(val1),
                op: Some(op),
                ..Default::default()
            };
        }
    }
    // 8. REFERENCE_REFERENCE: "<ref> <op> <ref>"
    if let Ok((ref1, op, ref2)) = sscanf!(form, "{}{}{}", String, char, String) {
        if ref1.chars().all(|c| c.is_ascii_alphanumeric()) &&
           ref2.chars().all(|c| c.is_ascii_alphanumeric()) {
            return ParsedFormula {
                formula_type: FormulaType::ReferenceReference,
                ref1: Some(ref1),
                ref2: Some(ref2),
                op: Some(op),
                ..Default::default()
            };
        }
    }
    // 9. RANGE_FUNCTION: "<func>(<ref1>:<ref2>)"
    if let Ok((func, ref1, ref2)) = sscanf!(form, "{}({}:{})", String, String, String) {
        return ParsedFormula {
            formula_type: FormulaType::RangeFunction,
            func: Some(func),
            ref1: Some(ref1),
            ref2: Some(ref2),
            ..Default::default()
        };
    }
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
        if r >= total_rows || c >= total_cols {
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
                            unsafe { STATUS_CODE = 3; }
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
            unsafe { STATUS_CODE = 3; }
            0
        }
    };
    
    if unsafe { EVAL_ERROR } || unsafe { STATUS_CODE } != 0 {
        err_value
    } else {
        CellValue::Int(result)
    }
}
