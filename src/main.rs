use std::{
    collections::HashSet,
    env,
    process,
};
#[cfg(not(feature = "gui"))]
use std::{
    io::{
        self,
        Write,
    },
    time::Instant,
};

#[cfg(feature = "gui")]
use eframe::egui;
#[cfg(feature = "gui")]
use gui_defs::SpreadsheetApp;

// Maximum length 7 bytes (e.g. "ZZZ999" is 6 characters; extra room for safety)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CellName {
    len:  u8,
    data: [u8; 7],
}

impl CellName {
    pub fn new(s: &str) -> Result<Self, &'static str> {
        if s.len() > 7 {
            return Err("CellName too long");
        }
        if !s.is_ascii() {
            return Err("CellName must be ASCII");
        }
        let mut data = [0u8; 7];
        data[..s.len()].copy_from_slice(s.as_bytes());
        Ok(CellName { len: s.len() as u8, data })
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.data[..self.len as usize]).unwrap()
    }
}

impl std::fmt::Display for CellName {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for CellName {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CellName::new(s)
    }
}
//////////////////////////////////////////////////////////////////////////////

#[cfg(feature = "gui")]
mod gui_defs;
#[cfg(feature = "gui")]
mod impl_helpers;
mod parser;
#[cfg(feature = "gui")]
mod render_gui;
#[cfg(feature = "gui")]
mod scroll_gui;
#[cfg(not(feature = "gui"))]
mod scrolling;
#[cfg(test)]
mod tests;
mod utils;
#[cfg(feature = "gui")]
mod utils_gui;

const STATUS: [&str; 4] = ["ok", "Invalid range", "unrecognized cmd", "cycle detected"];
pub static mut STATUS_CODE: usize = 0;

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

#[derive(Clone, PartialEq, Debug)]
pub enum Valtype {
    Int(i32),
    Str(CellName),
}

#[derive(Clone, Debug)]
pub enum CellData {
    Empty,
    SleepC,
    SleepR { cell1: CellName },
    Const,
    Ref { cell1: CellName },
    CoC { op_code: char, value2: Valtype },
    CoR { op_code: char, value2: Valtype, cell2: CellName },
    RoC { op_code: char, value2: Valtype, cell1: CellName },
    RoR { op_code: char, cell1: CellName, cell2: CellName },
    Range { cell1: CellName, cell2: CellName, value2: Valtype },
    Invalid,
}

#[derive(Clone)]
pub struct Cell {
    pub value:      Valtype,
    pub data:       CellData,
    pub dependents: HashSet<u32>,
}

impl Cell {
    pub fn reset(&mut self) {
        let current_dependents = std::mem::take(&mut self.dependents);
        *self = Self { value: Valtype::Int(0), data: CellData::Empty, dependents: current_dependents };
    }

    /// Clones a cell for backup without dependents.
    pub fn my_clone(&self) -> Self {
        Self {
            value:      self.value.clone(),
            data:       self.data.clone(),
            dependents: HashSet::new(), // intentionally not cloning dependents
        }
    }
}

#[cfg(not(feature = "gui"))]
fn print_sheet(
    spreadsheet: &[Vec<Cell>],
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
            if row < dimension.0 && col < spreadsheet[row].len() {
                match &spreadsheet[row][col].value {
                    Valtype::Int(v) => print!("{:<10}  ", v),
                    Valtype::Str(s) => print!("{:<10}         ", s),
                }
            } else {
                print!("{:<10}  ", 0);
            }
        }
        println!();
    }
}

fn parse_dimensions(args: Vec<String>) -> Result<(usize, usize), &'static str> {
    if args.len() == 4 && args[1] == "gui" {
        let total_rows = args[2].parse::<usize>().map_err(|_| "Invalid rows")?;
        let total_cols = args[3].parse::<usize>().map_err(|_| "Invalid columns")?;
        if !(1..=999).contains(&total_rows) || !(1..=18278).contains(&total_cols) {
            return Err("Invalid dimensions.");
        }
        Ok((total_rows, total_cols))
    } else if args.len() == 3 {
        let total_rows = args[1].parse::<usize>().map_err(|_| "Invalid rows")?;
        let total_cols = args[2].parse::<usize>().map_err(|_| "Invalid columns")?;
        if !(1..=999).contains(&total_rows) || !(1..=18278).contains(&total_cols) {
            return Err("Invalid dimensions.");
        }
        Ok((total_rows, total_cols))
    } else {
        return Err("Usage: <program> <num_rows> <num_columns>");
    }
}

#[cfg(not(feature = "gui"))]
fn interactive_mode(
    total_rows: usize,
    total_cols: usize,
) {
    let mut spreadsheet = vec![vec![Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() }; total_cols]; total_rows];

    let (mut start_row, mut start_col) = (0, 0);
    let mut enable_output = true;

    let prompt = |elapsed: f64, status: &str| {
        print!("[{:.1}] ({}) > ", elapsed, status);
        io::stdout().flush().unwrap();
    };

    let start_time = Instant::now();
    print_sheet(&spreadsheet, &(start_row, start_col), &(total_rows, total_cols));
    prompt(start_time.elapsed().as_secs_f64(), STATUS[unsafe { STATUS_CODE }]);

    let start = Instant::now();
    loop {
        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input).unwrap();
        if bytes_read == 0 {
            println!("Eval time: {:?}", start.elapsed());
            break;
        }
        println!();
        let start_time = Instant::now();
        let input = input.trim();
        unsafe {
            STATUS_CODE = 0;
        }
        match input {
            "w" => scrolling::w(&mut start_row),
            "s" => scrolling::s(&mut start_row, total_rows),
            "a" => scrolling::a(&mut start_col),
            "d" => scrolling::d(&mut start_col, total_cols),
            "q" => break,
            _ if input.contains('=') => {
                let parts: Vec<&str> = input.splitn(2, '=').map(str::trim).collect();
                if parts.len() == 2 {
                    let (cell_ref, formula) = (parts[0], parts[1]);
                    let (row, col) = utils::to_indices(cell_ref);
                    if row < total_rows && col < total_cols && unsafe { STATUS_CODE } == 0 {
                        let old_cell = spreadsheet[row][col].my_clone();
                        parser::detect_formula(&mut spreadsheet[row][col], formula);
                        parser::update_and_recalc(&mut spreadsheet, total_rows, total_cols, row, col, old_cell);
                    } else {
                        unsafe {
                            STATUS_CODE = 1;
                        }
                    }
                }
            }
            _ if input.starts_with("scroll_to ") => {
                let cell_ref = input.trim_start_matches("scroll_to ").trim();
                if cell_ref.is_empty() || !cell_ref.chars().next().unwrap().is_alphabetic() || scrolling::scroll_to(&mut start_row, &mut start_col, total_rows, total_cols, cell_ref).is_err() {
                    unsafe {
                        STATUS_CODE = 1;
                    }
                }
            }
            "disable_output" => enable_output = false,
            "enable_output" => enable_output = true,
            _ => unsafe {
                STATUS_CODE = 2;
            },
        }
        if enable_output {
            print_sheet(&spreadsheet, &(start_row, start_col), &(total_rows, total_cols));
        }
        prompt(start_time.elapsed().as_secs_f64(), STATUS[unsafe { STATUS_CODE }]);
    }
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
        let options = eframe::NativeOptions { viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 768.0]).with_resizable(true), ..Default::default() };
        eframe::run_native("Rust Spreadsheet", options, Box::new(move |_cc| Ok(Box::new(SpreadsheetApp::new(total_rows, total_cols, 0, 0))))).unwrap();
    }
    #[cfg(not(feature = "gui"))]
    interactive_mode(total_rows, total_cols);
}
