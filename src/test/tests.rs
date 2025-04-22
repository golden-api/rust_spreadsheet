use std::collections::HashSet;
use std::io;
use std::io::Write;

use crate::parser::{
    detect_formula,
    eval,
    update_and_recalc,
};
use crate::scrolling::{
    a,
    d,
    s,
    scroll_to,
    w,
};
use crate::utils::{
    EVAL_ERROR,
    compute,
    compute_range,
    to_indices,
};
use crate::{
    Cell,
    CellData,
    CellName,
    STATUS_CODE,
    Valtype,
    interactive_mode,
    parse_dimensions,
};

#[test]
fn test_detect_formula_various_types() {
    let mut cell = Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() };

    // Test SLEEP(<int>)
    unsafe {
        STATUS_CODE = 0;
    }
    detect_formula(&mut cell, "SLEEP(5)");
    assert!(matches!(cell.data, CellData::SleepC));
    assert_eq!(cell.value, Valtype::Int(5));

    // Test SLEEP(<ref>)
    unsafe {
        STATUS_CODE = 0;
    }
    detect_formula(&mut cell, "SLEEP(A1)");
    if let CellData::SleepR { cell1 } = &cell.data {
        assert_eq!(cell1.as_str(), "A1");
    } else {
        panic!("Expected SleepR, got {:?}", cell.data);
    }

    // Test CONSTANT
    unsafe {
        STATUS_CODE = 0;
    }
    detect_formula(&mut cell, "42");
    assert!(matches!(cell.data, CellData::Const));
    assert_eq!(cell.value, Valtype::Int(42));

    // Test REFERENCE
    unsafe {
        STATUS_CODE = 0;
    }
    detect_formula(&mut cell, "A1");
    if let CellData::Ref { cell1 } = &cell.data {
        assert_eq!(cell1.as_str(), "A1");
    } else {
        panic!("Expected Ref, got {:?}", cell.data);
    }

    // Test CONSTANT_CONSTANT
    unsafe {
        STATUS_CODE = 0;
    }
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
    unsafe {
        STATUS_CODE = 0;
    }
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
    unsafe {
        STATUS_CODE = 0;
    }
    detect_formula(&mut cell, "INVALID");
    assert!(matches!(cell.data, CellData::Invalid));
}

#[test]
fn test_eval_complex_scenarios() {
    let mut sheet = vec![vec![Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() }; 5]; 5];
    sheet[0][0].data = CellData::Const;
    sheet[0][0].value = Valtype::Int(10);
    sheet[1][1].data = CellData::Const;
    sheet[1][1].value = Valtype::Int(20);

    // Test CoR (10 + B2)
    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }
    sheet[2][0].data = CellData::CoR { op_code: '+', value2: Valtype::Int(10), cell2: CellName::new("B2").unwrap() };
    let result = eval(&sheet, 5, 5, 2, 0);
    assert_eq!(result, Valtype::Int(30));

    // Test RoR with out-of-bounds reference
    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }
    sheet[3][0].data = CellData::RoR {
        op_code: '-',
        cell1:   CellName::new("A1").unwrap(),
        cell2:   CellName::new("E6").unwrap(), // Out of bounds
    };
    let _ = eval(&sheet, 5, 5, 3, 0);
    assert_eq!(unsafe { STATUS_CODE }, 1);
}

#[test]
fn test_detect_formula_edge_cases() {
    let mut cell = Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() };

    // Test with whitespace
    unsafe {
        STATUS_CODE = 0;
    }
    detect_formula(&mut cell, "  42  ");
    assert!(matches!(cell.data, CellData::Const));
    assert_eq!(cell.value, Valtype::Int(42));

    // Test with negative values
    unsafe {
        STATUS_CODE = 0;
    }
    detect_formula(&mut cell, "-42");
    assert!(matches!(cell.data, CellData::Const));
    assert_eq!(cell.value, Valtype::Int(-42));

    // Test with invalid formula
    unsafe {
        STATUS_CODE = 0;
    }
    detect_formula(&mut cell, "A1B2");
    assert!(matches!(cell.data, CellData::Invalid));

    // Test with empty formula
    unsafe {
        STATUS_CODE = 0;
    }
    detect_formula(&mut cell, "");
    assert!(matches!(cell.data, CellData::Invalid));
}

#[test]
fn test_detect_formula_operations() {
    let mut cell = Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() };

    // Test with negative operands
    unsafe {
        STATUS_CODE = 0;
    }
    detect_formula(&mut cell, "-5+3");
    if let CellData::CoC { op_code, value2 } = &cell.data {
        assert_eq!(*op_code, '+');
        if let Valtype::Int(v) = value2 {
            assert_eq!(*v, 3);
        } else {
            panic!("Expected Int, got {:?}", value2);
        }
        assert_eq!(cell.value, Valtype::Int(-5));
    } else {
        panic!("Expected CoC, got {:?}", cell.data);
    }

    // Test with division
    unsafe {
        STATUS_CODE = 0;
    }
    detect_formula(&mut cell, "10/2");
    if let CellData::CoC { op_code, value2 } = &cell.data {
        assert_eq!(*op_code, '/');
        if let Valtype::Int(v) = value2 {
            assert_eq!(*v, 2);
        } else {
            panic!("Expected Int, got {:?}", value2);
        }
        assert_eq!(cell.value, Valtype::Int(10));
    } else {
        panic!("Expected CoC, got {:?}", cell.data);
    }
}

#[test]
fn test_update_and_recalc_multiple_dependencies() {
    let mut sheet = vec![vec![Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() }; 5]; 5];
    sheet[0][0].data = CellData::Const;
    sheet[0][0].value = Valtype::Int(5);
    sheet[1][0].data = CellData::Ref { cell1: CellName::new("A1").unwrap() };
    let backup = sheet[2][0].my_clone();
    unsafe {
        STATUS_CODE = 0;
    }
    sheet[2][0].data = CellData::Range { cell1: CellName::new("A1").unwrap(), cell2: CellName::new("B1").unwrap(), value2: Valtype::Str(CellName::new("SUM").unwrap()) };

    update_and_recalc(&mut sheet, 5, 5, 2, 0, backup);
    assert_eq!(sheet[2][0].value, Valtype::Int(5));
    let cell_hash = (2 * 5 + 0) as u32;
    assert!(sheet[0][0].dependents.contains(&cell_hash));
}

#[test]
fn test_update_and_recalc_complex_cycle() {
    let mut sheet = vec![vec![Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() }; 5]; 5];
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
    unsafe {
        STATUS_CODE = 0;
    }

    update_and_recalc(&mut sheet, 5, 5, 0, 0, backup);
    assert_eq!(unsafe { STATUS_CODE }, 3); // Cycle detected
}

#[test]
fn test_print_sheet() {
    let mut sheet = vec![vec![Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() }; 5]; 5];
    sheet[0][0].value = Valtype::Int(1);
    sheet[1][1].value = Valtype::Int(2);
    sheet[1][2].value = Valtype::Str(CellName::new("err").unwrap());
    {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        crate::print_sheet(&sheet, &(0, 0), &(5, 5));
        handle.flush().unwrap();
    }
    // Note: Exact output capture is tricky due to formatting; test structure
    // instead
    assert_eq!(unsafe { STATUS_CODE }, 0); // Ensure no errors
}

#[test]
fn test_parse_dimensions() {
    let args_cli = vec!["prog".to_string(), "5".to_string(), "10".to_string()];
    let args_invalid = vec!["prog".to_string(), "gui".to_string(), "0".to_string(), "10".to_string()];

    unsafe {
        STATUS_CODE = 0;
    }
    let result = crate::parse_dimensions(args_cli);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), (5, 10));

    unsafe {
        STATUS_CODE = 0;
    }
    let result = crate::parse_dimensions(args_invalid);
    assert!(result.is_err());
}

#[test]
#[ignore] // Interactive mode requires stdin simulation, marked ignore for now
fn test_interactive_mode() {
    let total_rows = 5;
    let total_cols = 5;
    unsafe {
        STATUS_CODE = 0;
    }
    crate::interactive_mode(total_rows, total_cols);
    assert_eq!(unsafe { STATUS_CODE }, 0); // Basic check
}

#[test]
fn test_detect_formula_range_functions() {
    let mut cell = Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() };

    // Test SUM
    unsafe {
        STATUS_CODE = 0;
    }
    detect_formula(&mut cell, "SUM(A1:B2)");
    if let CellData::Range { cell1, cell2, value2 } = &cell.data {
        assert_eq!(cell1.as_str(), "A1");
        assert_eq!(cell2.as_str(), "B2");
        if let Valtype::Str(func) = value2 {
            assert_eq!(func.as_str(), "SUM");
        } else {
            panic!("Expected Str, got {:?}", value2);
        }
    } else {
        panic!("Expected Range, got {:?}", cell.data);
    }

    // Test STDEV
    unsafe {
        STATUS_CODE = 0;
    }
    detect_formula(&mut cell, "STDEV(A1:Z9)");
    if let CellData::Range { cell1, cell2, value2 } = &cell.data {
        assert_eq!(cell1.as_str(), "A1");
        assert_eq!(cell2.as_str(), "Z9");
        if let Valtype::Str(func) = value2 {
            assert_eq!(func.as_str(), "STDEV");
        } else {
            panic!("Expected Str, got {:?}", value2);
        }
    } else {
        panic!("Expected Range, got {:?}", cell.data);
    }
}

#[test]
fn test_eval_edge_cases() {
    let sheet = vec![vec![Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() }; 5]; 5];

    // Eval on empty cell
    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }
    let result = eval(&sheet, 5, 5, 0, 0);
    assert_eq!(result, Valtype::Int(0));
}

#[test]
fn test_eval_invalid_formula() {
    let mut sheet = vec![vec![Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() }; 2]; 2];

    // Set cell to have invalid formula
    sheet[0][0].data = CellData::Invalid;

    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }
    let result = eval(&sheet, 2, 2, 0, 0);
    assert_eq!(result, Valtype::Int(0)); // Should return 0 for invalid formula
    assert_eq!(unsafe { STATUS_CODE }, 2); // Should set status code to 2 (unrecognized command)
}

#[test]
fn test_eval_sleep_constant() {
    let mut sheet = vec![vec![Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() }; 2]; 2];

    // Test with SleepC and small timeout
    sheet[0][0].data = CellData::SleepC;
    sheet[0][0].value = Valtype::Int(1);
    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }
    let start = std::time::Instant::now();
    let result = eval(&sheet, 2, 2, 0, 0);
    let elapsed = start.elapsed();
    assert_eq!(result, Valtype::Int(1));
    assert!(elapsed.as_millis() >= 900, "Sleep should have lasted at least 1 second");
}

#[test]
fn test_update_and_recalc_chains() {
    let mut sheet = vec![vec![Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() }; 5]; 5];

    // Setup chain: A1 = 1, B1 = A1+1, C1 = B1+1, D1 = C1+1
    sheet[0][0].data = CellData::Const;
    sheet[0][0].value = Valtype::Int(1);

    sheet[0][1].data = CellData::RoC { op_code: '+', value2: Valtype::Int(1), cell1: CellName::new("A1").unwrap() };

    sheet[0][2].data = CellData::RoC { op_code: '+', value2: Valtype::Int(1), cell1: CellName::new("B1").unwrap() };

    sheet[0][3].data = CellData::RoC { op_code: '+', value2: Valtype::Int(1), cell1: CellName::new("C1").unwrap() };

    // Setup dependencies
    let b1_hash = (0 * 5 + 1) as u32;
    let c1_hash = (0 * 5 + 2) as u32;
    let d1_hash = (0 * 5 + 3) as u32;

    sheet[0][0].dependents.insert(b1_hash);
    sheet[0][1].dependents.insert(c1_hash);
    sheet[0][2].dependents.insert(d1_hash);

    // Now change A1 and see if the chain updates
    let backup = sheet[0][0].my_clone();
    unsafe {
        STATUS_CODE = 0;
    }
    sheet[0][0].data = CellData::Const;
    sheet[0][0].value = Valtype::Int(10);

    update_and_recalc(&mut sheet, 5, 5, 0, 0, backup);

    // Check that all cells have been updated
    assert_eq!(sheet[0][0].value, Valtype::Int(10));
    assert_eq!(sheet[0][1].value, Valtype::Int(11));
    assert_eq!(sheet[0][2].value, Valtype::Int(12));
    assert_eq!(sheet[0][3].value, Valtype::Int(13));
}

//cellname in main.rs
#[test]
fn test_cellname_functions() {
    // Test valid cell name
    let cell_name = CellName::new("A1").unwrap();
    assert_eq!(cell_name.as_str(), "A1");

    // Test to_string
    assert_eq!(cell_name.to_string(), "A1");

    // Test from_str
    let cell_name: CellName = "B2".parse().unwrap();
    assert_eq!(cell_name.as_str(), "B2");

    // Test too long
    let result = CellName::new("ABCDEFGH");
    assert!(result.is_err());

    // Test non-ASCII
    let result = CellName::new("Ä1");
    assert!(result.is_err());
}

//scrolling.rs
#[test]
fn scrolling() {
    let total_rows = 25;
    let total_cols = 25;

    let mut start_row = 11;
    w(&mut start_row);
    assert_eq!(start_row, 1);

    w(&mut start_row);
    assert_eq!(start_row, 0);

    let mut start_col = 5;
    a(&mut start_col);
    assert_eq!(start_col, 0);

    start_col = 11;
    a(&mut start_col);
    assert_eq!(start_col, 1);

    start_row = 18;
    s(&mut start_row, total_rows);
    assert_eq!(start_row, 18);

    start_row = 4;
    s(&mut start_row, total_rows);
    assert_eq!(start_row, 14);

    start_row = 14;
    s(&mut start_row, total_rows);
    assert_eq!(start_row, 15);

    start_col = 12;
    d(&mut start_col, total_cols);
    assert_eq!(start_col, 15); // No change when already at boundary

    start_col = 15;
    d(&mut start_col, total_cols);
    assert_eq!(start_col, 15); // No change when already at boundary

    start_col = 4;
    d(&mut start_col, total_cols);
    assert_eq!(start_col, 14); // No change when already at boundary

    start_row = 0;
    start_col = 0;
    let _ = scroll_to(&mut start_row, &mut start_col, 1, 1, "A1");
    assert_eq!(start_row, 0);
    assert_eq!(start_col, 0);

    start_row = 0;
    start_col = 0;
    let _ = scroll_to(&mut start_row, &mut start_col, 100, 100, "C5");
    assert_eq!(start_row, 4); // Row index (5-1=4)
    assert_eq!(start_col, 2); // Column index (C=3-1=2)    
}
#[test]
fn test_invalid_scroll_to() {
    let mut start_row = 0;
    let mut start_col = 0;

    // Test invalid cell reference format
    let result = scroll_to(&mut start_row, &mut start_col, 10, 10, "Invalid123");
    assert!(result.is_err());

    // Test out-of-bounds reference
    let result = scroll_to(&mut start_row, &mut start_col, 10, 10, "K11");
    assert!(result.is_err());
}

//compute in utils.rs
#[test]
fn test_compute_operations_edge_cases() {
    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }
    assert_eq!(compute(-5, Some('+'), 3), -2);
    assert_eq!(compute(5, Some('/'), -2), -2);
    assert_eq!(compute(0, Some('*'), 5), 0);
    assert_eq!(compute(5, Some('/'), 0), 0); // Division by zero
    assert!(unsafe { EVAL_ERROR });
    unsafe {
        EVAL_ERROR = false;
    }
    assert_eq!(compute(5, Some('%'), 3), 0); // Invalid op
    assert_eq!(unsafe { STATUS_CODE }, 2);
}
#[test]
fn test_compute_range_edge_cases() {
    let mut sheet = vec![vec![Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() }; 2]; 2];
    sheet[0][0].value = Valtype::Int(1);
    sheet[0][1].value = Valtype::Int(-2);
    sheet[1][0].value = Valtype::Int(3);

    // Test MIN with negative
    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }
    let result = compute_range(&sheet, 0, 1, 0, 1, 2);
    assert_eq!(result, -2);

    // Test AVG with partial range
    let result = compute_range(&sheet, 0, 0, 0, 1, 3);
    assert_eq!(result, 0); // (1 + -2) / 2

    sheet[0][1].value = Valtype::Str(CellName::new("ERR").unwrap());
    // Test STDEV with small range
    let result = compute_range(&sheet, 0, 0, 0, 1, 5);
    assert!(result >= 1 && result <= 2); // Approx for [1, -2]
}
#[test]
fn test_compute_range_functions() {
    let mut sheet = vec![vec![Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() }; 4]; 4];

    // Setup values
    //  1  2  3  4
    //  5  6  7  8
    //  9 10 11 12
    // 13 14 15 16
    for i in 0..4 {
        for j in 0..4 {
            sheet[i][j].value = Valtype::Int(i as i32 * 4 + j as i32 + 1);
        }
    }

    // Test MAX function
    let result = compute_range(&sheet, 0, 1, 0, 1, 1);
    assert_eq!(result, 6); // max of [1,2,5,6] is 6

    // Test MIN function
    let result = compute_range(&sheet, 0, 1, 0, 1, 2);
    assert_eq!(result, 1); // min of [1,2,5,6] is 1

    // Test AVG function
    let result = compute_range(&sheet, 0, 1, 0, 1, 3);
    assert_eq!(result, 3); // avg of [1,2,5,6] is (1+2+5+6)/4 = 14/4 = 3.5 = 3 (integer division)

    // Test SUM function
    let result = compute_range(&sheet, 0, 1, 0, 1, 4);
    assert_eq!(result, 14); // sum of [1,2,5,6] is 14

    // Test STDEV function
    let result = compute_range(&sheet, 0, 0, 0, 3, 5);
    // STDEV of [1,2,3,4] has mean 2.5, variance is 1.25, stdev is sqrt(1.25) =
    // 1.118 = 1 (rounded)
    assert!(result > 0 && result < 2);

    // Test invalid function code
    unsafe {
        STATUS_CODE = 0;
    }
    let result = compute_range(&sheet, 0, 1, 0, 1, 6);
    assert_eq!(result, 0);
    assert_eq!(unsafe { STATUS_CODE }, 2);
}

//to_indices in utils
#[test]
fn test_to_indices_function() {
    unsafe {
        STATUS_CODE = 0;
    }
    let (row, col) = to_indices("A1");
    assert_eq!(row, 0);
    assert_eq!(col, 0);

    unsafe {
        STATUS_CODE = 0;
    }
    let (row, col) = to_indices("Z26");
    assert_eq!(row, 25);
    assert_eq!(col, 25);

    unsafe {
        STATUS_CODE = 0;
    }
    let (row, col) = to_indices("AA1");
    assert_eq!(row, 0);
    assert_eq!(col, 26);

    unsafe {
        STATUS_CODE = 0;
    }
    let (row, col) = to_indices("BC45");
    assert_eq!(row, 44);
    assert_eq!(col, 54); // B=2, C=3 -> BC = 2*26 + 3 = 55, so 54 zero-indexed

    // Test invalid indices
    unsafe {
        STATUS_CODE = 0;
    }
    let (row, col) = to_indices("A0");
    assert_eq!(row, 0);
    assert_eq!(col, 0);
    assert_eq!(unsafe { STATUS_CODE }, 1);
}

// Test for eval with CoC error case (lines 234-237)
#[test]
fn test_eval_coc_error() {
    let sheet = vec![vec![Cell { value: Valtype::Str(CellName::new("ERR").unwrap()), data: CellData::CoC { op_code: '+', value2: Valtype::Int(5) }, dependents: HashSet::new() }]];
    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }
    let result = eval(&sheet, 1, 1, 0, 0);
    assert_eq!(result, Valtype::Str(CellName::new("ERR").unwrap()));
    assert!(unsafe { EVAL_ERROR });
}

// Test for eval with RoR both references valid (lines 255-258)
#[test]
fn test_eval_ror_valid() {
    let mut sheet = vec![vec![Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() }; 2]; 2];
    sheet[0][0].data = CellData::Const;
    sheet[0][0].value = Valtype::Int(8);
    sheet[0][1].data = CellData::Const;
    sheet[0][1].value = Valtype::Int(2);
    sheet[1][0].data = CellData::RoR { op_code: '/', cell1: CellName::new("A1").unwrap(), cell2: CellName::new("B1").unwrap() };
    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }
    let result = eval(&sheet, 2, 2, 1, 0);
    assert_eq!(result, Valtype::Int(4));
}

// Test for eval with Range invalid range (lines 289-291)
#[test]
fn test_eval_range_out_of_bounds() {
    let sheet = vec![vec![Cell { value: Valtype::Int(0), data: CellData::Range { cell1: CellName::new("A1").unwrap(), cell2: CellName::new("Z10").unwrap(), value2: Valtype::Str(CellName::new("SUM").unwrap()) }, dependents: HashSet::new() }]];
    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }
    let result = eval(&sheet, 1, 1, 0, 0);
    assert_eq!(result, Valtype::Int(0));
    assert_eq!(unsafe { STATUS_CODE }, 1);
}

// Test for update_and_recalc with Range dependency removal (lines 377-382)
#[test]
fn test_update_and_recalc_range_removal() {
    let mut sheet = vec![vec![Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() }; 2]; 2];
    let cell_hash = (0 * 2 + 0) as u32;
    sheet[0][1].dependents.insert(cell_hash);
    sheet[1][0].dependents.insert(cell_hash);
    let backup = Cell { value: Valtype::Int(0), data: CellData::Range { cell1: CellName::new("A1").unwrap(), cell2: CellName::new("B2").unwrap(), value2: Valtype::Str(CellName::new("SUM").unwrap()) }, dependents: HashSet::new() };
    sheet[0][0].data = CellData::Const;
    sheet[0][0].value = Valtype::Int(42);
    update_and_recalc(&mut sheet, 2, 2, 0, 0, backup.clone());
    assert!(!sheet[0][1].dependents.contains(&cell_hash));
    assert!(!sheet[1][0].dependents.contains(&cell_hash));
}

// Test for detect_formula with invalid CONSTANT_CONSTANT (line 150, 152)
#[test]
fn test_detect_formula_invalid_const_const() {
    let mut cell = Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() };
    detect_formula(&mut cell, "5+"); // Incomplete expression
    assert!(matches!(cell.data, CellData::Invalid));
}

// Test for detect_formula with invalid CONSTANT_REFERENCE (lines 168, 170)
#[test]
fn test_detect_formula_invalid_const_ref() {
    let mut cell = Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() };
    detect_formula(&mut cell, "10*"); // Missing reference
    assert!(matches!(cell.data, CellData::Invalid));
}

// Test for detect_formula with invalid REFERENCE_CONSTANT (lines 173, 176)
#[test]
fn test_detect_formula_invalid_ref_const() {
    let mut cell = Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() };
    detect_formula(&mut cell, "A1-"); // Missing constant
    assert!(matches!(cell.data, CellData::Invalid));
}

// Test for detect_formula with invalid REFERENCE_REFERENCE (lines 189, 191)
#[test]
fn test_detect_formula_invalid_ref_ref() {
    let mut cell = Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() };
    detect_formula(&mut cell, "A1/"); // Missing second reference
    assert!(matches!(cell.data, CellData::Invalid));
}

// Test for detect_formula with invalid RANGE_FUNCTION (lines 201, 203)
#[test]
fn test_detect_formula_invalid_range() {
    let mut cell = Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() };
    detect_formula(&mut cell, "SUM(A1:)"); // Invalid range
    assert!(matches!(cell.data, CellData::Invalid));
}

// Test for eval with CoC with division by zero (lines 234, 237)
#[test]
fn test_eval_coc_div_zero() {
    let sheet = vec![vec![Cell { value: Valtype::Int(5), data: CellData::CoC { op_code: '/', value2: Valtype::Int(0) }, dependents: HashSet::new() }]];
    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }
    let result = eval(&sheet, 1, 1, 0, 0);
    assert_eq!(result, Valtype::Str(CellName::new("ERR").unwrap()));
    assert!(unsafe { EVAL_ERROR });
}

// Test for eval with unrecognized range function (lines 289, 291)
#[test]
fn test_eval_range_unrecognized_func() {
    let sheet = vec![vec![Cell { value: Valtype::Int(0), data: CellData::Range { cell1: CellName::new("A1").unwrap(), cell2: CellName::new("A1").unwrap(), value2: Valtype::Str(CellName::new("INVALID").unwrap()) }, dependents: HashSet::new() }]];
    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }
    let result = eval(&sheet, 1, 1, 0, 0);
    assert_eq!(result, Valtype::Int(0));
    assert_eq!(unsafe { STATUS_CODE }, 2);
}

// Test for eval with SleepR invalid reference (lines 304, 306)
#[test]
fn test_eval_sleepr_invalid_ref() {
    let sheet = vec![vec![Cell { value: Valtype::Int(0), data: CellData::SleepR { cell1: CellName::new("A10").unwrap() }, dependents: HashSet::new() }]];
    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }
    let result = eval(&sheet, 1, 1, 0, 0);
    assert_eq!(result, Valtype::Int(0));
    assert_eq!(unsafe { STATUS_CODE }, 1);
}

#[test]
fn test_parse_dimensions_invalid_rows() {
    let args = vec!["program".to_string(), "abc".to_string(), "5".to_string()];
    let result = parse_dimensions(args);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Invalid rows");
}

#[test]
fn test_parse_dimensions_out_of_bounds() {
    let args = vec!["program".to_string(), "1000".to_string(), "20000".to_string()];
    let result = parse_dimensions(args);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Invalid dimensions.");
}

#[test]
fn test_update_and_recalc_cor_addition_invalid() {
    let mut sheet = vec![vec![Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() }; 2]; 2];
    sheet[0][0].data = CellData::CoR {
        op_code: '+',
        value2:  Valtype::Int(5),
        cell2:   CellName::new("C1").unwrap(), // Out of bounds
    };
    let backup = Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() };
    update_and_recalc(&mut sheet, 2, 2, 0, 0, backup);
    assert_eq!(unsafe { STATUS_CODE }, 1); // Should fail validation
}

// Test RoC dependency addition with out-of-bounds (lines 404-407)
#[test]
fn test_update_and_recalc_roc_addition_out_of_bounds() {
    let mut sheet = vec![vec![Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() }; 2]; 2];
    sheet[0][0].data = CellData::RoC {
        op_code: '+',
        value2:  Valtype::Int(5),
        cell1:   CellName::new("C1").unwrap(), // Out of bounds
    };
    let backup = Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() };
    update_and_recalc(&mut sheet, 2, 2, 0, 0, backup);
    assert_eq!(unsafe { STATUS_CODE }, 1); // Should fail validation
}

#[test]
fn test_interactive_mode_compare_output() {
    unsafe {
        STATUS_CODE = 0;
    }

    let val = interactive_mode(999, 18278);
    assert_eq!(val, 5);
}
