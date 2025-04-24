use crate::CellData;
use crate::Valtype;

/// Converts a column index to an Excel-style label (e.g., 0 to "A", 1 to "B", 25 to "Z", 26 to "AA", etc.).
///
/// # Arguments
/// * `col_index` - The column index (0-based) to convert.
///
/// # Returns
/// A `String` representing the Excel-style column label.
///
/// # Examples
/// ```rust
/// assert_eq!(col_label(0), "A");
/// assert_eq!(col_label(1), "B");
/// assert_eq!(col_label(26), "AA");
/// ```
pub fn col_label(mut col_index: usize) -> String {
    let mut name = String::new();
    loop {
        let remainder = col_index % 26;
        name.insert(0, (b'A' + remainder as u8) as char);
        if col_index < 26 {
            break;
        }
        col_index = col_index / 26 - 1;
    }
    name
}

/// Parses a cell name (e.g., "A1", "AB78") into row and column indices.
///
/// The function expects a string with alphabetic characters for the column (e.g., "A", "AB")
/// followed by numeric characters for the row (e.g., "1", "78"). Non-alphanumeric characters
/// or invalid formats return `None`.
///
/// # Arguments
/// * `name` - The cell name to parse (e.g., "A1", "AB78").
///
/// # Returns
/// An `Option<(usize, usize)>` where the tuple contains (row_index, col_index), both 0-based.
/// Returns `None` if the format is invalid.
///
/// # Examples
/// ```rust
/// assert_eq!(parse_cell_name("A1"), Some((0, 0)));
/// assert_eq!(parse_cell_name("AB78"), Some((77, 27)));
/// assert_eq!(parse_cell_name("A"), None);
/// ```
pub fn parse_cell_name(name: &str) -> Option<(usize, usize)> {
    let mut col_part = String::new();
    let mut row_part = String::new();
    for c in name.chars() {
        if c.is_ascii_alphabetic() {
            col_part.push(c.to_ascii_uppercase());
        } else if c.is_ascii_digit() {
            row_part.push(c);
        } else {
            return None;
        }
    }
    if col_part.is_empty() || row_part.is_empty() {
        return None;
    }
    let col_index = col_label_to_index(&col_part)?;
    let row_index = row_part.parse::<usize>().ok()?.saturating_sub(1);
    Some((row_index, col_index))
}

/// Converts an Excel-style column label to a 0-based column index.
///
/// # Arguments
/// * `label` - The Excel-style column label (e.g., "A", "AB", "XYZ").
///
/// # Returns
/// An `Option<usize>` representing the 0-based column index, or `None` if the label contains
/// non-uppercase alphabetic characters.
///
/// # Examples
/// ```rust
/// assert_eq!(col_label_to_index("A"), Some(0));
/// assert_eq!(col_label_to_index("B"), Some(1));
/// assert_eq!(col_label_to_index("AA"), Some(26));
/// assert_eq!(col_label_to_index("a"), None); // Case-sensitive
/// ```
pub fn col_label_to_index(label: &str) -> Option<usize> {
    let mut col = 0;
    for (i, c) in label.chars().rev().enumerate() {
        if !c.is_ascii_uppercase() {
            return None;
        }
        col += ((c as u8 - b'A') as usize + 1) * 26_usize.pow(i as u32);
    }
    Some(col - 1)
}

/// Converts a `Valtype` to its string representation.
///
/// # Arguments
/// * `v` - The `Valtype` enum variant to convert (e.g., `Int` or `Str`).
///
/// # Returns
/// A `String` representing the value (e.g., "42" for `Int(42)`, "hello" for `Str("hello")`).
///
/// # Examples
/// ```rust
/// let int_val = Valtype::Int(42);
/// assert_eq!(valtype_to_string(&int_val), "42");
///
/// let str_val = Valtype::Str(String::from("hello"));
/// assert_eq!(valtype_to_string(&str_val), "hello");
/// ```
pub fn valtype_to_string(v: &Valtype) -> String {
    match v {
        Valtype::Int(n) => n.to_string(),
        Valtype::Str(s) => s.to_string(),
    }
}

/// Reconstructs an Excel-style formula from `CellData`.
///
/// Returns `None` if the cell has no formula (e.g., `Empty` or `Const`).
///
/// # Arguments
/// * `data` - The `CellData` to convert into a formula string.
///
/// # Returns
/// An `Option<String>` containing the formula (e.g., "=A1", "=1+2") if applicable, or `None`
/// for cells without formulas.
///
/// # Examples
/// ```rust
/// use crate::CellData;
/// use crate::Valtype;
///
/// let ref_data = CellData::Ref { cell1: String::from("A1") };
/// assert_eq!(cell_data_to_formula_string(&ref_data), Some(String::from("=A1")));
///
/// let empty_data = CellData::Empty;
/// assert_eq!(cell_data_to_formula_string(&empty_data), None);
/// ```
pub fn cell_data_to_formula_string(data: &CellData) -> Option<String> {
    use CellData::*;
    match data {
        Empty | Const => None,
        Ref { cell1 } => Some(format!("={}", cell1)),
        CoC { op_code, value2 } => Some(format!(
            "={}{}{}",
            /* left operand? */ "",
            op_code,
            valtype_to_string(value2)
        )),
        CoR {
            op_code,
            value2,
            cell2,
        } => Some(format!(
            "={}{}{}",
            valtype_to_string(value2),
            op_code,
            cell2
        )),
        RoC {
            op_code,
            value2,
            cell1,
        } => Some(format!(
            "={}{}{}",
            cell1,
            op_code,
            valtype_to_string(value2)
        )),
        RoR {
            op_code,
            cell1,
            cell2,
        } => Some(format!("={}{}{}", cell1, op_code, cell2)),
        Range {
            cell1,
            cell2,
            value2,
        } => Some(format!(
            "=RANGE({}:{},{})",
            cell1,
            cell2,
            valtype_to_string(value2)
        )),
        SleepC => Some("=SLEEP()".into()),
        SleepR { cell1 } => Some(format!("=SLEEP({})", cell1)),
        Invalid => Some("#INVALID".into()),
    }
}
