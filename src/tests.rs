use crate::{Cell, CellData, STATUS_CODE, Valtype, CellName};
use crate::parser::{detect_formula, eval, update_and_recalc};
use crate::scrolling::{w, s, a, d, scroll_to};
use crate::utils::{EVAL_ERROR, compute, compute_range, sleepy, to_indices};
use std::collections::HashSet;

#[test]
fn test_detect_formula_various_types() {
    let mut cell = Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    };

    // Test SLEEP(<int>)
    unsafe { STATUS_CODE = 0; }
    detect_formula(&mut cell, "SLEEP(5)");
    assert!(matches!(cell.data, CellData::SleepC));
    assert_eq!(cell.value, Valtype::Int(5));

    // Test SLEEP(<ref>)
    unsafe { STATUS_CODE = 0; }
    detect_formula(&mut cell, "SLEEP(A1)");
    if let CellData::SleepR { cell1 } = &cell.data {
        assert_eq!(cell1.as_str(), "A1");
    } else {
        panic!("Expected SleepR, got {:?}", cell.data);
    }

    // Test CONSTANT
    unsafe { STATUS_CODE = 0; }
    detect_formula(&mut cell, "42");
    assert!(matches!(cell.data, CellData::Const));
    assert_eq!(cell.value, Valtype::Int(42));

    // Test REFERENCE
    unsafe { STATUS_CODE = 0; }
    detect_formula(&mut cell, "A1");
    if let CellData::Ref { cell1 } = &cell.data {
        assert_eq!(cell1.as_str(), "A1");
    } else {
        panic!("Expected Ref, got {:?}", cell.data);
    }

    // Test CONSTANT_CONSTANT
    unsafe { STATUS_CODE = 0; }
    detect_formula(&mut cell, "5+3");
    if let CellData::CoC { op_code, value2 } = &cell.data {
        assert_eq!(*op_code, '+');
        if let Valtype::Int(v) = value2 {
            assert_eq!(*v, 3);
        } else {
            panic!("Expected Int, got {:?}", value2);
        }
    } else {
        panic!("Expected CoC, got {:?}", cell.data);
    }

    // Test RANGE
    unsafe { STATUS_CODE = 0; }
    detect_formula(&mut cell, "MAX(A1:B2)");
    if let CellData::Range { cell1, cell2, value2 } = &cell.data {
        assert_eq!(cell1.as_str(), "A1");
        assert_eq!(cell2.as_str(), "B2");
        if let Valtype::Str(func) = value2 {
            assert_eq!(func.as_str(), "MAX");
        } else {
            panic!("Expected Str, got {:?}", value2);
        }
    } else {
        panic!("Expected Range, got {:?}", cell.data);
    }
}

#[test]
fn test_eval_basic_computations() {
    let mut sheet = vec![vec![Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    }; 10]; 10];
    sheet[0][0].data = CellData::Const;
    sheet[0][0].value = Valtype::Int(5);
    sheet[0][1].data = CellData::Const;
    sheet[0][1].value = Valtype::Int(3);

    // Test CoC (5 + 3)
    unsafe { STATUS_CODE = 0; EVAL_ERROR = false; }
    sheet[1][0].data = CellData::CoC { op_code: '+', value2: Valtype::Int(3) };
    sheet[1][0].value = Valtype::Int(5);
    let result = eval(&sheet, 10, 10, 1, 0);
    assert_eq!(result, Valtype::Int(8));

    // Test Ref
    unsafe { STATUS_CODE = 0; EVAL_ERROR = false; }
    sheet[2][0].data = CellData::Ref { cell1: CellName::new("A1").unwrap() };
    let result = eval(&sheet, 10, 10, 2, 0);
    assert_eq!(result, Valtype::Int(5));

    // Test Range (SUM)
    unsafe { STATUS_CODE = 0; EVAL_ERROR = false; }
    sheet[3][0].data = CellData::Range {
        cell1: CellName::new("A1").unwrap(),
        cell2: CellName::new("B1").unwrap(),
        value2: Valtype::Str(CellName::new("SUM").unwrap()),
    };
    let result = eval(&sheet, 10, 10, 3, 0);
    assert_eq!(result, Valtype::Int(8));
}

#[test]
fn test_update_and_recalc_no_cycle() {
    let mut sheet = vec![vec![Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    }; 10]; 10];
    sheet[0][0].value = Valtype::Int(5);
    sheet[0][0].data = CellData::Const;
    let backup = sheet[1][0].my_clone();
    unsafe { STATUS_CODE = 0; }
    sheet[1][0].data = CellData::Ref { cell1: CellName::new("A1").unwrap() };

    update_and_recalc(&mut sheet, 10, 10, 1, 0, backup);
    assert_eq!(sheet[1][0].value, Valtype::Int(5));
    let cell_hash = (1 * 10 + 0) as u32;
    assert!(sheet[0][0].dependents.contains(&cell_hash));
}

#[test]
fn test_update_and_recalc_cycle_detection() {
    
}

#[test]
fn test_scroll_functions() {
    let mut start_row = 50;
    let mut start_col ;
    let total_rows = 100;
    let total_cols = 100;

    // Test w
    w(&mut start_row);
    assert_eq!(start_row, 40);

    // Test s
    start_row = 80;
    s(&mut start_row, total_rows);
    assert_eq!(start_row, 90);

    // Test a
    start_col = 40;
    a(&mut start_col);
    assert_eq!(start_col, 30);

    // Test d
    start_col = 80;
    d(&mut start_col, total_cols);
    assert_eq!(start_col, 90);
}

#[test]
fn test_scroll_to_valid_and_invalid() {
    let mut start_row = 0;
    let mut start_col = 0;
    let total_rows = 10;
    let total_cols = 10;

    // Valid scroll
    let result = scroll_to(&mut start_row, &mut start_col, total_rows, total_cols, "B2");
    assert!(result.is_ok());
    assert_eq!(start_row, 1);
    assert_eq!(start_col, 1);

    // Invalid scroll (out of bounds)
    let result = scroll_to(&mut start_row, &mut start_col, total_rows, total_cols, "J11");
    assert!(result.is_err());
    assert_eq!(start_row, 1); // Should not change
    assert_eq!(start_col, 1); // Should not change
}

#[test]
fn test_to_indices() {
    unsafe { STATUS_CODE = 0; }
    assert_eq!(to_indices("A1"), (0, 0));
    assert_eq!(to_indices("B2"), (1, 1));
    assert_eq!(to_indices("AA10"), (9, 26));
}

#[test]
fn test_compute_operations() {
    unsafe { STATUS_CODE = 0; EVAL_ERROR = false; }
    assert_eq!(compute(5, Some('+'), 3), 8);
    assert_eq!(compute(5, Some('-'), 3), 2);
    assert_eq!(compute(5, Some('*'), 3), 15);
    assert_eq!(compute(5, Some('/'), 0), 0); // Division by zero
    assert!(unsafe { EVAL_ERROR });
    unsafe { EVAL_ERROR = false; }
    assert_eq!(compute(5, None, 3), 0); // Invalid op
    assert_eq!(unsafe { STATUS_CODE }, 2);
}

#[test]
fn test_sleepy() {
    let start = std::time::Instant::now();
    sleepy(1); // Sleep for 1 second
    assert!(start.elapsed().as_secs() >= 1);
}

#[test]
fn test_compute_range() {
    let mut sheet = vec![vec![Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    }; 3]; 3];
    sheet[0][0].value = Valtype::Int(1);
    sheet[0][1].value = Valtype::Int(2);
    sheet[1][0].value = Valtype::Int(3);
    sheet[1][1].value = Valtype::Int(4);

    // Test SUM
    unsafe { STATUS_CODE = 0; EVAL_ERROR = false; }
    let result = compute_range(&sheet, 0, 1, 0, 1, 4);
    assert_eq!(result, 10); // 1 + 2 + 3 + 4

    // Test MAX
    let result = compute_range(&sheet, 0, 1, 0, 1, 1);
    assert_eq!(result, 4);

    // Test AVG
    let result = compute_range(&sheet, 0, 1, 0, 1, 3);
    assert_eq!(result, 2); // (10 / 4)

    // Test STDEV (approximate due to rounding)
    let result = compute_range(&sheet, 0, 1, 0, 1, 5);
    assert!(result >= 1 && result <= 2); // Rough estimate for std dev of [1, 2, 3, 4]
}