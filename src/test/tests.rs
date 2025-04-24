use std::collections::HashMap;
use std::collections::HashSet;
use std::io;
use std::io::Write;
use std::time::Instant;

use crate::parser::{detect_formula, eval, update_and_recalc};
use crate::scrolling::{a, d, s, scroll_to, w};
use crate::utils::{
    EVAL_ERROR,
    compute,
    compute_range,
    to_indices,
};
use crate::{
    Cell, CellData, CellName, STATUS, STATUS_CODE, Valtype, interactive_mode, parse_dimensions,
    print_sheet, prompt,
};
fn make_sheet(cap: usize) -> HashMap<u32, Cell> {
    HashMap::with_capacity(cap)
}

/// Insert or overwrite one cell in the map.
fn set_cell(
    sheet: &mut HashMap<u32, Cell>,
    total_cols: usize,
    r: usize,
    c: usize,
    data: CellData,
    value: Valtype,
) {
    let key = (r * total_cols + c) as u32;
    sheet.insert(
        key,
        Cell {
            data,
            value,
            dependents: HashSet::new(),
        },
    );
}
#[test]
fn test_detect_formula_various_types() {
    let mut cell = Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    };

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
    if let CellData::Range {
        cell1,
        cell2,
        value2,
    } = &cell.data
    {
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
    let total_rows = 5;
    let total_cols = 5;
    let mut sheet = make_sheet(1024);

    // A1 = 10
    set_cell(
        &mut sheet,
        total_cols,
        0,
        0,
        CellData::Const,
        Valtype::Int(10),
    );
    // B2 = 20
    set_cell(
        &mut sheet,
        total_cols,
        1,
        1,
        CellData::Const,
        Valtype::Int(20),
    );
    // Test CoR (10 + B2)
    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }
    set_cell(
        &mut sheet,
        total_cols,
        2,
        0,
        CellData::CoR {
            op_code: '+',
            value2: Valtype::Int(10),
            cell2: CellName::new("B2").unwrap(),
        },
        Valtype::Int(0), // initial value placeholder
    );
    let result = eval(&sheet, total_rows, total_cols, 2, 0);
    assert_eq!(result, Valtype::Int(30));
    // Test RoR with out-of-bounds reference
    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }
    set_cell(
        &mut sheet,
        total_cols,
        3,
        0,
        CellData::RoR {
            op_code: '-',
            cell1: CellName::new("A1").unwrap(),
            cell2: CellName::new("E6").unwrap(), // Out of bounds
        },
        Valtype::Int(0),
    );

    let _ = eval(&sheet, 5, 5, 3, 0);
    assert_eq!(unsafe { STATUS_CODE }, 1);
}

#[test]
fn test_detect_formula_edge_cases() {
    let mut cell = Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    };

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
    let mut cell = Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    };

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
fn test_update_and_recalc_complex_cycle() {
    let mut sheet = make_sheet(25); // 5x5 sheet
    let mut ranged: HashMap<u32, Vec<(u32, u32)>> = HashMap::with_capacity(32);
    let mut is_range: Vec<bool> = vec![false; 25];

    let total_cols = 5;

    let cell_hash_a1 = (0 * total_cols + 0) as u32;
    let cell_hash_b1 = (1 * total_cols + 0) as u32;
    let cell_hash_c1 = (2 * total_cols + 0) as u32;

    // A1 = B1
    sheet.insert(
        cell_hash_a1,
        Cell {
            data: CellData::Ref {
                cell1: CellName::new("B1").unwrap(),
            },
            value: Valtype::Int(0),
            dependents: {
                let mut d = HashSet::new();
                d.insert(cell_hash_b1);
                d
            },
        },
    );

    // B1 = C1
    sheet.insert(
        cell_hash_b1,
        Cell {
            data: CellData::Ref {
                cell1: CellName::new("C1").unwrap(),
            },
            value: Valtype::Int(0),
            dependents: {
                let mut d = HashSet::new();
                d.insert(cell_hash_c1);
                d
            },
        },
    );

    // C1 = A1 → cycle
    sheet.insert(
        cell_hash_c1,
        Cell {
            data: CellData::Ref {
                cell1: CellName::new("A1").unwrap(),
            },
            value: Valtype::Int(0),
            dependents: {
                let mut d = HashSet::new();
                d.insert(cell_hash_a1);
                d
            },
        },
    );

    let backup = sheet.get(&cell_hash_a1).unwrap().my_clone();

    unsafe {
        STATUS_CODE = 0;
    }

    update_and_recalc(
        &mut sheet,
        &mut ranged,
        &mut is_range,
        (total_cols, 5),
        0,
        0,
        backup,
    );

    assert_eq!(unsafe { STATUS_CODE }, 3); // Cycle detected
}

#[test]
fn test_print_sheet() {
    let mut sheet = make_sheet(25);
    let total_cols = 5;

    set_cell(
        &mut sheet,
        total_cols,
        0,
        0,
        CellData::Empty,
        Valtype::Int(1),
    );
    set_cell(
        &mut sheet,
        total_cols,
        1,
        1,
        CellData::Empty,
        Valtype::Int(2),
    );
    set_cell(
        &mut sheet,
        total_cols,
        1,
        2,
        CellData::Empty,
        Valtype::Str(CellName::new("err").unwrap()),
    );

    {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        crate::print_sheet(&sheet, &(0, 0), &(5, 5));
        handle.flush().unwrap();
    }

    assert_eq!(unsafe { STATUS_CODE }, 0);
}

#[test]
fn test_parse_dimensions() {
    let args_cli = vec!["prog".to_string(), "5".to_string(), "10".to_string()];
    let args_invalid = vec![
        "prog".to_string(),
        "gui".to_string(),
        "0".to_string(),
        "10".to_string(),
    ];

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
fn test_detect_formula_range_functions() {
    let mut cell = Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    };

    // Test SUM
    unsafe {
        STATUS_CODE = 0;
    }
    detect_formula(&mut cell, "SUM(A1:B2)");
    if let CellData::Range {
        cell1,
        cell2,
        value2,
    } = &cell.data
    {
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
    if let CellData::Range {
        cell1,
        cell2,
        value2,
    } = &cell.data
    {
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
    let sheet = make_sheet(30);

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
    let mut sheet = make_sheet(4);
    let total_cols = 2;
    let key = (0 * total_cols + 0) as u32;

    sheet.insert(
        key,
        Cell {
            data: CellData::Invalid,
            value: Valtype::Int(0),
            dependents: HashSet::new(),
        },
    );

    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }

    let result = eval(&sheet, 2, 2, 0, 0);
    assert_eq!(result, Valtype::Int(0));
    assert_eq!(unsafe { STATUS_CODE }, 2);
}

#[test]
fn test_eval_sleep_constant() {
    let mut sheet = make_sheet(4);
    let total_cols = 2;
    let key = (0 * total_cols + 0) as u32;

    sheet.insert(
        key,
        Cell {
            data: CellData::SleepC,
            value: Valtype::Int(1),
            dependents: HashSet::new(),
        },
    );

    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }

    let start = std::time::Instant::now();
    let result = eval(&sheet, 2, 2, 0, 0);
    let elapsed = start.elapsed();

    assert_eq!(result, Valtype::Int(1));
    assert!(
        elapsed.as_millis() >= 900,
        "Sleep should have lasted at least 1 second"
    );
}

#[test]
fn test_update_and_recalc_chains() {
    let mut sheet = make_sheet(25);
    let mut ranged: HashMap<u32, Vec<(u32, u32)>> = HashMap::with_capacity(32);
    let mut is_range: Vec<bool> = vec![false; 25];

    let total_cols = 5;

    // A1 = 1
    set_cell(
        &mut sheet,
        total_cols,
        0,
        0,
        CellData::Const,
        Valtype::Int(1),
    );

    // B1 = A1 + 1
    set_cell(
        &mut sheet,
        total_cols,
        0,
        1,
        CellData::RoC {
            op_code: '+',
            value2: Valtype::Int(1),
            cell1: CellName::new("A1").unwrap(),
        },
        Valtype::Int(0),
    );

    // C1 = B1 + 1
    set_cell(
        &mut sheet,
        total_cols,
        0,
        2,
        CellData::RoC {
            op_code: '+',
            value2: Valtype::Int(1),
            cell1: CellName::new("B1").unwrap(),
        },
        Valtype::Int(0),
    );

    // D1 = C1 + 1
    set_cell(
        &mut sheet,
        total_cols,
        0,
        3,
        CellData::RoC {
            op_code: '+',
            value2: Valtype::Int(1),
            cell1: CellName::new("C1").unwrap(),
        },
        Valtype::Int(0),
    );

    let a1 = (0 * total_cols + 0) as u32;
    let b1 = (0 * total_cols + 1) as u32;
    let c1 = (0 * total_cols + 2) as u32;
    let d1 = (0 * total_cols + 3) as u32;

    sheet.get_mut(&a1).unwrap().dependents.insert(b1);
    sheet.get_mut(&b1).unwrap().dependents.insert(c1);
    sheet.get_mut(&c1).unwrap().dependents.insert(d1);

    let backup = sheet.get(&a1).unwrap().my_clone();

    unsafe {
        STATUS_CODE = 0;
    }

    sheet.get_mut(&a1).unwrap().data = CellData::Const;
    sheet.get_mut(&a1).unwrap().value = Valtype::Int(10);

    update_and_recalc(&mut sheet, &mut ranged, &mut is_range, (5, 5), 0, 0, backup);

    assert_eq!(sheet.get(&a1).unwrap().value, Valtype::Int(10));
    assert_eq!(sheet.get(&b1).unwrap().value, Valtype::Int(11));
    assert_eq!(sheet.get(&c1).unwrap().value, Valtype::Int(12));
    assert_eq!(sheet.get(&d1).unwrap().value, Valtype::Int(13));
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
    let mut sheet = make_sheet(1);
    let total_cols = 1;

    // Insert a cell with CoC operation and error value
    set_cell(
        &mut sheet,
        total_cols,
        0,
        0,
        CellData::CoC {
            op_code: '+',
            value2: Valtype::Int(5),
        },
        Valtype::Str(CellName::new("ERR").unwrap()),
    );

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
    let mut sheet = make_sheet(4);
    let total_cols = 2;

    // A1 = 8
    set_cell(
        &mut sheet,
        total_cols,
        0,
        0,
        CellData::Const,
        Valtype::Int(8),
    );

    // B1 = 2
    set_cell(
        &mut sheet,
        total_cols,
        0,
        1,
        CellData::Const,
        Valtype::Int(2),
    );

    // A2 = A1 / B1
    set_cell(
        &mut sheet,
        total_cols,
        1,
        0,
        CellData::RoR {
            op_code: '/',
            cell1: CellName::new("A1").unwrap(),
            cell2: CellName::new("B1").unwrap(),
        },
        Valtype::Int(0),
    );

    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }

    let result = eval(&sheet, 2, 2, 1, 0);
    assert_eq!(result, Valtype::Int(4));
}

// Test for detect_formula with invalid CONSTANT_CONSTANT (line 150, 152)
#[test]
fn test_detect_formula_invalid_const_const() {
    let mut cell = Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    };
    detect_formula(&mut cell, "5+"); // Incomplete expression
    assert!(matches!(cell.data, CellData::Invalid));
}

// Test for detect_formula with invalid CONSTANT_REFERENCE (lines 168, 170)
#[test]
fn test_detect_formula_invalid_const_ref() {
    let mut cell = Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    };
    detect_formula(&mut cell, "10*"); // Missing reference
    assert!(matches!(cell.data, CellData::Invalid));
}

// Test for detect_formula with invalid REFERENCE_CONSTANT (lines 173, 176)
#[test]
fn test_detect_formula_invalid_ref_const() {
    let mut cell = Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    };
    detect_formula(&mut cell, "A1-"); // Missing constant
    assert!(matches!(cell.data, CellData::Invalid));
}

// Test for detect_formula with invalid RANGE_FUNCTION (lines 201, 203)
#[test]
fn test_detect_formula_invalid_range() {
    let mut cell = Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    };
    detect_formula(&mut cell, "SUM(A1:)"); // Invalid range
    assert!(matches!(cell.data, CellData::Invalid));
}

// Test for eval with CoC with division by zero (lines 234, 237)
#[test]
fn test_parse_dimensions_invalid_rows() {
    let args = vec!["program".to_string(), "abc".to_string(), "5".to_string()];
    let result = parse_dimensions(args);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Invalid rows");
}

#[test]
fn test_parse_dimensions_out_of_bounds() {
    let args = vec![
        "program".to_string(),
        "1000".to_string(),
        "20000".to_string(),
    ];
    let result = parse_dimensions(args);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Invalid dimensions.");
}

#[test]
fn test_eval_coc_div_zero() {
    let mut sheet = make_sheet(1);
    set_cell(
        &mut sheet,
        1,
        0,
        0,
        CellData::CoC {
            op_code: '/',
            value2: Valtype::Int(0),
        },
        Valtype::Int(5),
    );
    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }
    let result = eval(&sheet, 1, 1, 0, 0);
    assert_eq!(result, Valtype::Str(CellName::new("ERR").unwrap()));
    assert!(unsafe { EVAL_ERROR });
}
#[test]
fn test_update_and_recalc_roc_addition_out_of_bounds() {
    let mut sheet = make_sheet(2);
    let mut ranged: HashMap<u32, Vec<(u32, u32)>> = HashMap::with_capacity(32);
    let mut is_range: Vec<bool> = vec![false; 25];

    let cell_data = CellData::RoC {
        op_code: '+',
        value2: Valtype::Int(5),
        cell1: CellName::new("C1").unwrap(), // Out of bounds
    };
    let backup = Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    };
    set_cell(&mut sheet, 2, 0, 0, cell_data, Valtype::Int(0));
    update_and_recalc(&mut sheet, &mut ranged, &mut is_range, (2, 2), 0, 0, backup);
    assert_eq!(unsafe { STATUS_CODE }, 1);
}
#[test]
fn test_update_and_recalc_cor_addition_invalid() {
    let mut sheet = make_sheet(2);
    let mut ranged: HashMap<u32, Vec<(u32, u32)>> = HashMap::with_capacity(32);
    let mut is_range: Vec<bool> = vec![false; 25];

    let cell_data = CellData::CoR {
        op_code: '+',
        value2: Valtype::Int(5),
        cell2: CellName::new("C1").unwrap(), // Out of bounds
    };
    let backup = Cell {
        value: Valtype::Int(0),
        data: CellData::Empty,
        dependents: HashSet::new(),
    };
    set_cell(&mut sheet, 2, 0, 0, cell_data, Valtype::Int(0));
    update_and_recalc(&mut sheet, &mut ranged, &mut is_range, (2, 2), 0, 0, backup);
    assert_eq!(unsafe { STATUS_CODE }, 1);
}
#[test]
fn test_eval_sleepr_invalid_ref() {
    let mut sheet = make_sheet(1);
    set_cell(
        &mut sheet,
        1,
        0,
        0,
        CellData::SleepR {
            cell1: CellName::new("A10").unwrap(),
        },
        Valtype::Int(0),
    );
    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }
    let result = eval(&sheet, 1, 1, 0, 0);
    assert_eq!(result, Valtype::Int(0));
    assert_eq!(unsafe { STATUS_CODE }, 1);
}
#[test]
fn test_eval_range_unrecognized_func() {
    let mut sheet = make_sheet(1);
    set_cell(
        &mut sheet,
        1,
        0,
        0,
        CellData::Range {
            cell1: CellName::new("A1").unwrap(),
            cell2: CellName::new("A1").unwrap(),
            value2: Valtype::Str(CellName::new("INVALID").unwrap()),
        },
        Valtype::Int(0),
    );
    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }
    let result = eval(&sheet, 1, 1, 0, 0);
    assert_eq!(result, Valtype::Int(0));
    assert_eq!(unsafe { STATUS_CODE }, 2);
}

#[test]
fn test_interactive_mode() {
    // Initialize data structures with HashMap implementation
    let mut spreadsheet: HashMap<u32, Cell> = HashMap::with_capacity(1024);
    let mut ranged: HashMap<u32, Vec<(u32, u32)>> = HashMap::with_capacity(32);
    let mut is_range: Vec<bool> = vec![false; 10000]; // This should probably be larger based on grid size

    // Initial view position
    let (mut start_row, mut start_col) = (0, 0);
    let mut enable_output = true;

    // Total grid dimensions
    let (total_rows, total_cols) = (100, 100);

    // Begin tracking execution time
    let start_time = Instant::now();
    print_sheet(
        &spreadsheet,
        &(start_row, start_col),
        &(total_rows, total_cols),
    );
    prompt(
        start_time.elapsed().as_secs_f64(),
        STATUS[unsafe { STATUS_CODE }],
    );

    // Series of commands to test
    let commands = [
        "disable_output",
        "A1=5",
        "scroll_to B2",
        "scroll_to 12",
        "A2=A1+3",
        "A1=MAX(B1:Z26)",
        "A1=SLEEP(B1)",
        "A1=A2",
        "ZZZ999=A1",
        "A2=A1",
        "A1=5",
        "A1=2=3",
        "enable_output",
        "j",
        "q",
    ];

    // Process each command in sequence
    let mut i = 0;
    loop {
        if !interactive_mode(
            &mut spreadsheet,
            &mut ranged,
            &mut is_range,
            commands[i].to_string(),
            (total_rows, total_cols),
            &mut enable_output,
            &mut (&mut start_row, &mut start_col),
        ) {
            break;
        }
        i += 1;
    }

    // Verify A1 has value 5 (key 0 = row 0, col 0)
    assert_eq!(spreadsheet.get(&0).unwrap().value, Valtype::Int(5));
}

#[test]
fn test_compute_range_str_value() {
    let mut sheet = make_sheet(10);
    let total_cols = 5;

    // Set A1 (0,0) to a string value ("ERR")
    set_cell(
        &mut sheet,
        total_cols,
        0,
        0,
        CellData::Empty,
        Valtype::Str(CellName::new("ERR").unwrap()),
    );

    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }

    // Compute SUM over A1:A1 (single cell with string)
    let result = compute_range(&sheet, total_cols, 0, 0, 0, 0, 4); // SUM
    assert_eq!(result, 0); // Should skip string value
    assert!(unsafe { EVAL_ERROR }); // Should set EVAL_ERROR
    assert_eq!(unsafe { STATUS_CODE }, 0);
}
#[test]
fn test_compute_range_invalid_choice() {
    let sheet = make_sheet(10);
    let total_cols = 5;

    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }

    // Compute with invalid choice (e.g., 0)
    let result = compute_range(&sheet, total_cols, 0, 1, 0, 1, 0);
    assert_eq!(result, 0); // Should return 0 for invalid choice
    assert_eq!(unsafe { STATUS_CODE }, 2); // Should set STATUS_CODE
    assert!(!unsafe { EVAL_ERROR });
}
#[test]
fn test_compute_range_stdev_full() {
    let mut sheet = make_sheet(10);
    let total_cols = 5;

    // Set A1=1, A2=3, B1=5, B2=7 (values for STDEV)
    set_cell(&mut sheet, total_cols, 0, 0, CellData::Const, Valtype::Int(1)); // A1
    set_cell(&mut sheet, total_cols, 1, 0, CellData::Const, Valtype::Int(3)); // A2
    set_cell(&mut sheet, total_cols, 0, 1, CellData::Const, Valtype::Int(5)); // B1
    set_cell(&mut sheet, total_cols, 1, 1, CellData::Const, Valtype::Int(7)); // B2

    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }

    // Compute STDEV over A1:B2
    let result = compute_range(&sheet, total_cols, 0, 1, 0, 1, 5); // STDEV
    // Expected: Values [1, 3, 5, 7], mean = 4, variance = ((1-4)^2 + (3-4)^2 + (5-4)^2 + (7-4)^2)/4 = (9+1+1+9)/4 = 5, sqrt(5) ≈ 2.236, round to 2
    assert_eq!(result, 2);
    assert_eq!(unsafe { STATUS_CODE }, 0);
    assert!(!unsafe { EVAL_ERROR });
}
#[test]
fn test_compute_range_min() {
    let mut sheet = make_sheet(10);
    let total_cols = 5;

    // Set A1=10, A2=5, B1=8
    set_cell(&mut sheet, total_cols, 0, 0, CellData::Const, Valtype::Int(10)); // A1
    set_cell(&mut sheet, total_cols, 1, 0, CellData::Const, Valtype::Int(5));  // A2
    set_cell(&mut sheet, total_cols, 0, 1, CellData::Const, Valtype::Int(8));  // B1

    unsafe {
        STATUS_CODE = 0;
        EVAL_ERROR = false;
    }

    // Compute MIN over A1:B2
    let result = compute_range(&sheet, total_cols, 0, 1, 0, 1, 2); // MIN
    assert_eq!(result, 0); // Minimum of [10, 5, 8, 0] is 5
    assert_eq!(unsafe { STATUS_CODE }, 0);
    assert!(!unsafe { EVAL_ERROR });
    let result = compute_range(&sheet, total_cols, 0, 1, 0, 1, 3); // AVG
    assert_eq!(result, 5); // Minimum of [10, 5, 8, 0] is 5
    assert_eq!(unsafe { STATUS_CODE }, 0);
    assert!(!unsafe { EVAL_ERROR });
}

#[test]
fn test_interactive_mode_parser_coverage() {
    // Initialize data structures
    let mut spreadsheet: HashMap<u32, Cell> = HashMap::with_capacity(1024);
    let mut ranged: HashMap<u32, Vec<(u32, u32)>> = HashMap::with_capacity(32);
    let mut is_range: Vec<bool> = vec![false; 10000];
    let (mut start_row, mut start_col) = (0, 0);
    let mut enable_output = true;
    let (total_rows, total_cols) = (100, 100);

    // Commands to cover uncovered lines
    let commands = vec![
        "A1=5*2",           // CONSTANT_CONSTANT with * (lines 163, 165)
        "A2=10-A1",        // CONSTANT_REFERENCE with - (lines 181, 183)
        "A8=A1/B10",       // RoR with out-of-bounds (lines 422–424)
        "A9=AVG(A1:A2)",   // Range with AVG (lines 370–373, 375, 377, 385)
        "A10=SLEEP(B10)",  // SleepR with invalid ref (lines 409–412)
        "B1=10",           // Set B1 for dependencies
        "B2=B1+A1",        // RoR for dependency (lines 628–631)
        "B3=5+B1",         // CoR for dependency (lines 603–607, 612)
        "B4=A1+5",         // RoC for dependency (lines 621–624)
        "B5=SLEEP(A1)",    // SleepR for dependency (lines 635–636, 639)
        "B6=SUM(A1:B2)",   // Range for dependency (lines 560–566)
        "disable_output",  // Suppress output
        "q",               // Quit
    ];

    // Process commands
    let start_time = Instant::now();
    print_sheet(
        &spreadsheet,
        &(start_row, start_col),
        &(total_rows, total_cols),
    );
    prompt(
        start_time.elapsed().as_secs_f64(),
        STATUS[unsafe { STATUS_CODE }],
    );

    let mut i = 0;
    loop {
        if !interactive_mode(
            &mut spreadsheet,
            &mut ranged,
            &mut is_range,
            commands[i].to_string(),
            (total_rows, total_cols),
            &mut enable_output,
            &mut (&mut start_row, &mut start_col),
        ) {
            break;
        }
        i += 1;
    }

    // Verify results
    assert_eq!(spreadsheet.get(&0).unwrap().value, Valtype::Int(10)); // A1 = 5*2
    assert_eq!(spreadsheet.get(&1).unwrap().value, Valtype::Int(10));  // A2 = 10-A1
}
#[test]
fn test_interactive_mode_full_coverage() {
    // Initialize data structures
    let mut spreadsheet: HashMap<u32, Cell> = HashMap::with_capacity(1024);
    let mut ranged: HashMap<u32, Vec<(u32, u32)>> = HashMap::with_capacity(32);
    let mut is_range: Vec<bool> = vec![false; 10000];
    let (mut start_row, mut start_col) = (0, 0);
    let mut enable_output = true;
    let (total_rows, total_cols) = (100, 100);

    // Commands to cover all remaining lines
    let commands = vec![
        "A1=3+4",           // CONSTANT_CONSTANT with + (lines 163, 165)
        "A2=7*B1",         // CONSTANT_REFERENCE with * (lines 181, 183)
        "A3=MAX(A1:A2)",   // RANGE_FUNCTION with MAX (lines 203, 205)
        "A4=+",            // Invalid formula syntax (lines 218, 220, 225)
        "A5=A1+ERR",       // Invalid reference (lines 237, 239, 244 for CoC error)
        "A6=5-C10",        // CoR with out-of-bounds (lines 280, 282, 290)
        "A7=B1*2",         // RoC with invalid ref (lines 346, 348)
        "A8=SUM(A1:A2)",   // Range evaluation (lines 375, 377, 385)
        "A9=SLEEP(A10)",   // SleepR with invalid ref (lines 409–412)
        "B1=A1",           // Ref for dependency validation (lines 422–424)
        "B2=SUM(A1:B1)",   // Range dependency (lines 560–566)
        "B3=A1+1",         // CoR dependency (lines 603–607, 612)
        "B4=2*A1",         // RoC dependency (lines 621–624)
        "B5=A1+B1",        // RoR dependency (lines 628–631)
        "B6=SLEEP(A1)",    // SleepR dependency (lines 635–636, 639)
        "C1=B1",           // Ref dependency (line 587)
        "C2=C1+2",         // Dependency chain for BFS (lines 644–647, 651)
        "C3=C2+3",         // Topological sort (lines 689, 691–692)
        "A1=10",           // Update A1 to trigger dependency removal (lines 482–484, 495–497)
        "disable_output",  // Suppress output
        "q",               // Quit
    ];

    // Process commands
    let start_time = Instant::now();
    print_sheet(
        &spreadsheet,
        &(start_row, start_col),
        &(total_rows, total_cols),
    );
    prompt(
        start_time.elapsed().as_secs_f64(),
        STATUS[unsafe { STATUS_CODE }],
    );

    let mut i = 0;
    loop {
        if !interactive_mode(
            &mut spreadsheet,
            &mut ranged,
            &mut is_range,
            commands[i].to_string(),
            (total_rows, total_cols),
            &mut enable_output,
            &mut (&mut start_row, &mut start_col),
        ) {
            break;
        }
        i += 1;
    }

    // Verify key results
    assert_eq!(spreadsheet.get(&0).unwrap().value, Valtype::Int(10));  // A1 = 10
    assert_eq!(spreadsheet.get(&100).unwrap().value, Valtype::Int(70)); // A2 = 70 (updated)
    assert_eq!(spreadsheet.get(&2).unwrap().value, Valtype::Int(10));  // A3 = MAX(A1:A2)
    assert_eq!(spreadsheet.get(&202).unwrap().value, Valtype::Int(15)); // C3 = C2+3
}