/// # Rust Spreadsheet
/// This is the main entry point for the Rust Spreadsheet application.
/// It supports both interactive command-line mode (default) and GUI mode (when the "gui" feature is enabled).
/// The application processes command-line arguments to set up the spreadsheet dimensions and delegates to
/// either `interactive_mode` or a GUI interface based on configuration.
use std::{
    collections::{HashMap, HashSet},
    env, process,
};
#[cfg(not(feature = "gui"))]
use std::{
    io::{self, Write},
    time::Instant,
};

#[cfg(feature = "gui")]
use eframe::egui;
#[cfg(feature = "gui")]
use gui::gui_defs::SpreadsheetApp;

/// A compact representation of a cell reference (e.g., "A1") with a maximum length of 7 bytes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CellName {
    len: u8,
    data: [u8; 7],
}

impl CellName {
    /// Creates a new `CellName` from a string.
    ///
    /// # Arguments
    /// * `s` - The string representation of the cell (e.g., "A1").
    ///
    /// # Returns
    /// * `Result<Self, &'static str>` - Success with a `CellName` or an error message if the input is invalid.
    ///
    /// # Errors
    /// * Returns `Err` if the string is longer than 7 characters or contains non-ASCII characters.
    pub fn new(s: &str) -> Result<Self, &'static str> {
        if s.len() > 7 {
            return Err("CellName too long");
        }
        if !s.is_ascii() {
            return Err("CellName must be ASCII");
        }
        let mut data = [0u8; 7];
        data[..s.len()].copy_from_slice(s.as_bytes());
        Ok(CellName {
            len: s.len() as u8,
            data,
        })
    }
    /// Returns the string representation of the `CellName`.
    ///
    /// # Returns
    /// * `&str` - The string representation of the cell reference.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.data[..self.len as usize]).unwrap()
    }
}

impl std::fmt::Display for CellName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for CellName {
    type Err = &'static str;
    /// Parses a string into a `CellName`.
    ///
    /// # Arguments
    /// * `s` - The string to parse.
    ///
    /// # Returns
    /// * `Result<Self, Self::Err>` - Success with a `CellName` or an error if parsing fails.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CellName::new(s)
    }
}
////////////////////////////////////////////////////////////////////////////////

mod parser;
#[cfg(not(feature = "gui"))]
mod scrolling;

#[cfg(feature = "gui")]
mod gui;
mod test;
mod utils;
/// Array of status messages used to indicate the outcome of operations.
const STATUS: [&str; 4] = ["ok", "Invalid range", "unrecognized cmd", "cycle detected"];
/// A global variable to store the current status code (0-3).
/// Use with `unsafe` due to its mutable global nature.
pub static mut STATUS_CODE: usize = 0;
/// Represents the type of formula a cell can contain.
pub enum FormulaType {
    SleepC,
    SleepR,
    Const,
    Ref,
    CoR,
    RoC,
    CoC,
    RoR,
    Range,
    Invalid,
}
/// Represents the value of a cell, which can be either an integer or a string (for errors).
#[derive(Clone, PartialEq, Debug)]
pub enum Valtype {
    Int(i32),
    Str(CellName),
}
/// Represents the type of data stored in a cell, including constants, references, and operations.
#[derive(Clone, Debug, PartialEq)]
pub enum CellData {
    Empty,
    SleepC,
    SleepR {
        cell1: CellName,
    },
    Const,
    Ref {
        cell1: CellName,
    },
    CoC {
        op_code: char,
        value2: Valtype,
    },
    CoR {
        op_code: char,
        value2: Valtype,
        cell2: CellName,
    },
    RoC {
        op_code: char,
        value2: Valtype,
        cell1: CellName,
    },
    RoR {
        op_code: char,
        cell1: CellName,
        cell2: CellName,
    },
    Range {
        cell1: CellName,
        cell2: CellName,
        value2: Valtype,
    },
    Invalid,
}
/// Represents a cell in the spreadsheet, containing its value, data type, and dependents.
#[derive(Clone)]
pub struct Cell {
    pub value: Valtype,
    pub data: CellData,
    pub dependents: HashSet<u32>,
}

impl Cell {
    /// Resets the cell to its default state, preserving its dependents.
    pub fn reset(&mut self) {
        let current_dependents = std::mem::take(&mut self.dependents);
        *self = Self {
            value: Valtype::Int(0),
            data: CellData::Empty,
            dependents: current_dependents,
        };
    }

    /// Clones a cell for backup without copying its dependents.
    ///
    /// # Returns
    /// * `Self` - A new `Cell` with the same value and data, but an empty set of dependents.
    pub fn my_clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            data: self.data.clone(),
            dependents: HashSet::new(), // intentionally not cloning dependents
        }
    }
}

#[cfg(not(feature = "gui"))]
/// Prints the spreadsheet grid starting from the given position.
///
/// # Arguments
/// * `spreadsheet` - A hash map containing cell data, indexed by a unique `u32` key.
/// * `pointer` - A tuple `(row, col)` indicating the starting position to display.
/// * `dimension` - A tuple `(total_rows, total_cols)` defining the spreadsheet dimensions.
fn print_sheet(
    spreadsheet: &HashMap<u32, Cell>,
    pointer: &(usize, usize),
    dimension: &(usize, usize),
) {
    let view_rows = dimension.0.saturating_sub(pointer.0).min(10);
    let view_cols = dimension.1.saturating_sub(pointer.1).min(10);

    print!("{:<5}", "");
    for j in 0..view_cols {
        let col = pointer.1 + j;
        let mut name = String::new();
        let mut n = col + 1;
        while n > 0 {
            let rem = (n - 1) % 26;
            name.push((b'A' + rem as u8) as char);
            n = (n - 1) / 26;
        }
        print!("{:>10}  ", name.chars().rev().collect::<String>());
    }
    println!();

    for i in 0..view_rows {
        print!("{:4}  ", pointer.0 + i + 1);
        for j in 0..view_cols {
            let row = pointer.0 + i;
            let col = pointer.1 + j;
            let idx = (row as u32) * (dimension.1 as u32) + (col as u32);
            let cell = spreadsheet.get(&idx).cloned().unwrap_or(Cell {
                value: Valtype::Int(0),
                data: CellData::Empty,
                dependents: HashSet::new(),
            });
            match &cell.value {
                Valtype::Int(v) => print!("{:<10}  ", v),
                Valtype::Str(s) => print!("{:<10}         ", s),
            }
        }
        println!();
    }
}
/// Parses command-line arguments to determine spreadsheet dimensions.
///
/// # Arguments
/// * `args` - A vector of command-line arguments.
///
/// # Returns
/// * `Result<(usize, usize), &'static str>` - A tuple `(rows, cols)` on success, or an error message on failure.
fn parse_dimensions(args: Vec<String>) -> Result<(usize, usize), &'static str> {
    if args.len() == 3 {
        let total_rows = args[1].parse::<usize>().map_err(|_| "Invalid rows")?;
        let total_cols = args[2].parse::<usize>().map_err(|_| "Invalid columns")?;
        if !(1..=999).contains(&total_rows) || !(1..=18278).contains(&total_cols) {
            return Err("Invalid dimensions.");
        }
        Ok((total_rows, total_cols))
    } else {
        Err("Usage: <program> <num_rows> <num_columns>")
    }
}

#[cfg(not(feature = "gui"))]
/// Processes a single input command in interactive mode, updating the spreadsheet state.
///
/// # Arguments
/// * `spreadsheet` - A hash map containing cell data, indexed by a unique `u32` key.
/// * `ranged` - A hash map tracking ranges for dependency management.
/// * `is_range` - A boolean array indicating whether each cell is part of a range.
/// * `input` - The user input command to process.
/// * `total_dims` - A tuple `(total_rows, total_cols)` defining the spreadsheet dimensions.
/// * `enable_output` - A mutable boolean controlling whether to print the spreadsheet after each command.
/// * `start_dims` - A mutable tuple `(&mut start_row, &mut start_col)` defining the current view position.
///
/// # Returns
/// * `bool` - `true` to continue the interactive loop, `false` to exit.
fn interactive_mode(
    spreadsheet: &mut HashMap<u32, Cell>,
    ranged: &mut HashMap<u32, Vec<(u32, u32)>>,
    is_range: &mut [bool],
    input: String,
    total_dims: (usize, usize),
    enable_output: &mut bool,
    start_dims: &mut (&mut usize, &mut usize),
) -> bool {
    println!();
    let start_time = Instant::now();
    let input = input.trim();
    unsafe {
        STATUS_CODE = 0;
    }
    let (total_rows, total_cols) = total_dims;
    //let (start_row, start_col) = start_dims;
    match input {
        "w" => scrolling::w(start_dims.0),
        "s" => scrolling::s(start_dims.0, total_rows),
        "a" => scrolling::a(start_dims.1),
        "d" => scrolling::d(start_dims.1, total_cols),
        "q" => return false,
        _ if input.contains('=') => {
            let parts: Vec<&str> = input.splitn(2, '=').map(str::trim).collect();
            if parts.len() == 2 {
                let (cell_ref, formula) = (parts[0], parts[1]);
                let (row, col) = utils::to_indices(cell_ref);
                if row < total_rows && col < total_cols && unsafe { STATUS_CODE } == 0 {
                    let idx = (row as u32) * (total_cols as u32) + (col as u32);
                    let old_cell = spreadsheet.get(&idx).cloned().unwrap_or(Cell {
                        value: Valtype::Int(0),
                        data: CellData::Empty,
                        dependents: HashSet::new(),
                    });
                    let mut new_cell = old_cell.clone();
                    parser::detect_formula(&mut new_cell, formula);
                    spreadsheet.insert(idx, new_cell);
                    parser::update_and_recalc(
                        spreadsheet,
                        ranged,
                        is_range,
                        (total_rows, total_cols),
                        row,
                        col,
                        old_cell,
                    );
                } else {
                    unsafe {
                        STATUS_CODE = 1;
                    }
                }
            }
        }
        _ if input.starts_with("scroll_to ") => {
            let cell_ref = input.trim_start_matches("scroll_to ").trim();
            if cell_ref.is_empty()
                || !cell_ref.chars().next().unwrap().is_alphabetic()
                || scrolling::scroll_to(
                    start_dims.0,
                    start_dims.1,
                    total_rows,
                    total_cols,
                    cell_ref,
                )
                .is_err()
            {
                unsafe {
                    STATUS_CODE = 1;
                }
            }
        }
        "disable_output" => *enable_output = false,
        "enable_output" => *enable_output = true,
        _ => unsafe {
            STATUS_CODE = 2;
        },
    }
    if *enable_output {
        print_sheet(
            spreadsheet,
            &(*start_dims.0, *start_dims.1),
            &(total_rows, total_cols),
        );
    }
    prompt(
        start_time.elapsed().as_secs_f64(),
        STATUS[unsafe { STATUS_CODE }],
    );
    true
}
#[cfg(not(feature = "gui"))]
/// Prints the command prompt with elapsed time and status.
///
/// # Arguments
/// * `elapsed` - The elapsed time in seconds since the last command.
/// * `status` - The current status message.
fn prompt(elapsed: f64, status: &str) {
    print!("[{:.1}] ({}) > ", elapsed, status);
    io::stdout().flush().unwrap();
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let (total_rows, total_cols) = match parse_dimensions(args.clone()) {
        Ok(dim) => dim,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    #[cfg(feature = "gui")]
    {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1024.0, 768.0])
                .with_resizable(true),
            ..Default::default()
        };
        eframe::run_native(
            "Rust Spreadsheet",
            options,
            Box::new(move |_cc| Ok(Box::new(SpreadsheetApp::new(total_rows, total_cols, 0, 0)))),
        )
        .unwrap();
    }
    #[cfg(not(feature = "gui"))]
    {
        let mut spreadsheet: HashMap<u32, Cell> = HashMap::with_capacity(1024);
        let mut ranged: HashMap<u32, Vec<(u32, u32)>> = HashMap::with_capacity(512);
        let mut is_range: Vec<bool> = vec![false; total_rows * total_cols];
        let mut start_row = 0;
        let mut start_col = 0;
        let mut enable_output = true;
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
        loop {
            let mut input = String::new();
            let bytes_read = io::stdin().read_line(&mut input).unwrap();
            if bytes_read == 0 {
                break;
            }
            if !interactive_mode(
                &mut spreadsheet,
                &mut ranged,
                &mut is_range,
                input,
                (total_rows, total_cols),
                &mut enable_output,
                &mut (&mut start_row, &mut start_col),
            ) {
                break;
            }
        }
    }
}
