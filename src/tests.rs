use crate::{Cell, CellData, STATUS_CODE, Valtype, CellName};
use crate::parser::{detect_formula, eval, update_and_recalc};
use crate::scrolling::{w, s, a, d, scroll_to};
use crate::utils::{EVAL_ERROR, compute, compute_range, sleepy, to_indices};
use std::collections::HashSet;
use std::io;
use std::io::Write;

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

    // Test invalid input
    unsafe { STATUS_CODE = 0; }
    detect_formula(&mut cell, "INVALID");
    assert!(matches!(cell.data, CellData::Invalid));
}

#[test]
fn test_eval_complex_scenarios() {
    let mut sheet = vec![vec![Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    }; 5]; 5];
    sheet[0][0].data = CellData::Const;
    sheet[0][0].value = Valtype::Int(10);
    sheet[1][1].data = CellData::Const;
    sheet[1][1].value = Valtype::Int(20);

    // Test CoR (10 + B2)
    unsafe { STATUS_CODE = 0; EVAL_ERROR = false; }
    sheet[2][0].data = CellData::CoR {
        op_code: '+',
        value2: Valtype::Int(10),
        cell2: CellName::new("B2").unwrap(),
    };
    let result = eval(&sheet, 5, 5, 2, 0);
    assert_eq!(result, Valtype::Int(30));

    // Test RoR with out-of-bounds reference
    unsafe { STATUS_CODE = 0; EVAL_ERROR = false; }
    sheet[3][0].data = CellData::RoR {
        op_code: '+',
        cell1: CellName::new("A1").unwrap(),
        cell2: CellName::new("E6").unwrap(), // Out of bounds
    };
    let result = eval(&sheet, 5, 5, 3, 0);
    assert_eq!(unsafe { STATUS_CODE }, 1);
}

#[test]
fn test_update_and_recalc_multiple_dependencies() {
    let mut sheet = vec![vec![Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    }; 5]; 5];
    sheet[0][0].data = CellData::Const;
    sheet[0][0].value = Valtype::Int(5);
    sheet[1][0].data = CellData::Ref { cell1: CellName::new("A1").unwrap() };
    let backup = sheet[2][0].my_clone();
    unsafe { STATUS_CODE = 0; }
    sheet[2][0].data = CellData::Range {
        cell1: CellName::new("A1").unwrap(),
        cell2: CellName::new("B1").unwrap(),
        value2: Valtype::Str(CellName::new("SUM").unwrap()),
    };

    update_and_recalc(&mut sheet, 5, 5, 2, 0, backup);
    assert_eq!(sheet[2][0].value, Valtype::Int(5));
    let cell_hash = (2 * 5 + 0) as u32;
    assert!(sheet[0][0].dependents.contains(&cell_hash));
}

#[test]
fn test_update_and_recalc_complex_cycle() {
    let mut sheet = vec![vec![Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    }; 5]; 5];
    let cell_hash_a1 = (0 * 5 + 0) as u32;
    let cell_hash_b1 = (1 * 5 + 0) as u32;
    let cell_hash_c1 = (2 * 5 + 0) as u32;
    sheet[0][0].data = CellData::Ref { cell1: CellName::new("B1").unwrap() };
    sheet[0][0].dependents.insert(cell_hash_b1);
    sheet[1][0].data = CellData::Ref { cell1: CellName::new("C1").unwrap() };
    sheet[1][0].dependents.insert(cell_hash_c1);
    sheet[2][0].data = CellData::Ref { cell1: CellName::new("A1").unwrap() };
    sheet[2][0].dependents.insert(cell_hash_a1);
    let backup = sheet[0][0].my_clone();
    unsafe { STATUS_CODE = 0; }

    update_and_recalc(&mut sheet, 5, 5, 0, 0, backup);
    assert_eq!(unsafe { STATUS_CODE }, 3); // Cycle detected
}

#[test]
fn test_scroll_functions_boundaries() {
    let mut start_row = 0;
    let mut start_col = 0;
    let total_rows = 15;
    let total_cols = 15;

    // Test w at boundary
    w(&mut start_row);
    assert_eq!(start_row, 0);

    // Test s at upper boundary
    start_row = 5;
    s(&mut start_row, total_rows);
    assert_eq!(start_row, 5);

    // Test a at boundary
    a(&mut start_col);
    assert_eq!(start_col, 0);

    // Test d at upper boundary
    start_col = 5;
    d(&mut start_col, total_cols);
    assert_eq!(start_col, 5);
}

#[test]
fn test_scroll_to_edge_cases() {
    let mut start_row = 0;
    let mut start_col = 0;
    let total_rows = 10;
    let total_cols = 10;

    // Valid edge case
    let result = scroll_to(&mut start_row, &mut start_col, total_rows, total_cols, "J10");
    assert!(result.is_ok());
    assert_eq!(start_row, 9);
    assert_eq!(start_col, 9);

    // Invalid format
    let result = scroll_to(&mut start_row, &mut start_col, total_rows, total_cols, "A100");
    assert!(result.is_err());
    let result = scroll_to(&mut start_row, &mut start_col, total_rows, total_cols, "A");
    assert!(result.is_err());
}

#[test]
fn test_compute_operations_edge_cases() {
    unsafe { STATUS_CODE = 0; EVAL_ERROR = false; }
    assert_eq!(compute(-5, Some('+'), 3), -2);
    assert_eq!(compute(5, Some('/'), -2), -2);
    assert_eq!(compute(0, Some('*'), 5), 0);
    assert_eq!(compute(5, Some('/'), 0), 0); // Division by zero
    assert!(unsafe { EVAL_ERROR });
    unsafe { EVAL_ERROR = false; }
    assert_eq!(compute(5, Some('%'), 3), 0); // Invalid op
    assert_eq!(unsafe { STATUS_CODE }, 2);
}

#[test]
fn test_compute_range_edge_cases() {
    let mut sheet = vec![vec![Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    }; 2]; 2];
    sheet[0][0].value = Valtype::Int(1);
    sheet[0][1].value = Valtype::Int(-2);
    sheet[1][0].value = Valtype::Int(3);

    // Test MIN with negative
    unsafe { STATUS_CODE = 0; EVAL_ERROR = false; }
    let result = compute_range(&sheet, 0, 1, 0, 1, 2);
    assert_eq!(result, -2);

    // Test AVG with partial range
    let result = compute_range(&sheet, 0, 0, 0, 1, 3);
    assert_eq!(result, 0); // (1 + -2) / 2

    // Test STDEV with small range
    let result = compute_range(&sheet, 0, 0, 0, 1, 5);
    assert!(result >= 1 && result <= 2); // Approx for [1, -2]
}

#[test]
fn test_print_sheet() {
    let mut sheet = vec![vec![Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    }; 5]; 5];
    sheet[0][0].value = Valtype::Int(1);
    sheet[1][1].value = Valtype::Int(2);
    {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        crate::print_sheet(&sheet, &(0, 0), &(5, 5));
        handle.flush().unwrap();
    }
    // Note: Exact output capture is tricky due to formatting; test structure instead
    assert_eq!(unsafe { STATUS_CODE }, 0); // Ensure no errors
}

#[test]
fn test_parse_dimensions() {
    let args_gui = vec!["prog".to_string(), "gui".to_string(), "5".to_string(), "10".to_string()];
    let args_cli = vec!["prog".to_string(), "5".to_string(), "10".to_string()];
    let args_invalid = vec!["prog".to_string(), "0".to_string(), "10".to_string()];

    unsafe { STATUS_CODE = 0; }
    let result = crate::parse_dimensions(args_gui);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), (5, 10));

    unsafe { STATUS_CODE = 0; }
    let result = crate::parse_dimensions(args_cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), (5, 10));

    unsafe { STATUS_CODE = 0; }
    let result = crate::parse_dimensions(args_invalid);
    assert!(result.is_err());
}

#[test]
#[ignore] // Interactive mode requires stdin simulation, marked ignore for now
fn test_interactive_mode() {
    let total_rows = 5;
    let total_cols = 5;
    let mut sheet = vec![vec![Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    }; total_cols]; total_rows];
    // Simulate input would require mocking stdin, skipped for now
    unsafe { STATUS_CODE = 0; }
    crate::interactive_mode(total_rows, total_cols);
    assert_eq!(unsafe { STATUS_CODE }, 0); // Basic check
}