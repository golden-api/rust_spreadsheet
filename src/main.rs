mod utils;
mod scrolling;
mod parser;
mod gui;
use utils::{to_indices, STATUS_CODE,update_cell};
use scrolling::*;
use parser::recalc;
use std::{collections::HashSet, env, io::{self, Write}, process, time::Instant};

const MAX_ROWS:u8 = 10;
const MAX_COLS:u8 = 10;
const STATUS: [&str; 4] = ["ok", "Invalid range", "unrecognized cmd", "cycle detected"];

#[derive(Clone)]
pub enum CellValue {
    Int(i32),
    Str(String),
}

#[derive(Clone)]
pub struct Cell {
    pub value: CellValue,
    pub formula: Option<String>,
    pub dependents: HashSet<(u16, u16)>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            value: CellValue::Int(0),
            formula: None,
            dependents: HashSet::new(),
        }
    }
}

fn col_name(col: usize) -> String {
    let mut n = col + 1;
    let mut result = String::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        result.push((b'A' + rem as u8) as char);
        n = (n - 1) / 26;
    }
    result.chars().rev().collect()
}

fn printsheet(spreadsheet: &[Vec<Cell>], start_row: usize, start_col: usize, total_rows: usize, total_cols: usize) {
    let view_rows = total_rows.saturating_sub(start_row).min(MAX_ROWS as usize);
    let view_cols = total_cols.saturating_sub(start_col).min(MAX_COLS as usize);
    print!("{:<5}", "");
    for j in 0..view_cols {
        print!("{:>10}  ", col_name(start_col + j));
    }
    println!();
    for i in 0..view_rows {
        print!("{:4}  ", start_row + i + 1);
        for j in 0..view_cols {
            if start_row + i < spreadsheet.len() && start_col + j < spreadsheet[start_row + i].len() {
                let cell = &spreadsheet[start_row + i][start_col + j];
                match &cell.value {
                    CellValue::Int(v) => print!("{:<10}  ", v),
                    CellValue::Str(s) => print!("{:<10}  ", s),
                }
            } else {
                print!("{:<10}  ", 0);
            }
        }
        println!();
    }
}

fn parse_dimensions(args: Vec<String>) -> Result<(usize, usize), &'static str> {
    if args.len() != 3 {
        return Err("Usage: <program> <num_rows> <num_columns>");
    }
    let total_rows = args[1].parse::<usize>().map_err(|_| "Invalid rows")?;
    let total_cols = args[2].parse::<usize>().map_err(|_| "Invalid columns")?;
    if !(1..=999).contains(&total_rows) || !(1..=18278).contains(&total_cols) {
        return Err("Invalid dimensions.");
    }
    Ok((total_rows, total_cols))
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() >= 3 && args[1] == "gui" {
        let rows = args.get(2).and_then(|s| s.parse::<usize>().ok()).unwrap_or(20);
        let cols = args.get(3).and_then(|s| s.parse::<usize>().ok()).unwrap_or(50);
        gui::run_gui(rows, cols);
        return;
    }
    let (total_rows, total_cols) = match parse_dimensions(args) {
        Ok(dim) => dim,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };
    let mut visited = vec![0u8; total_rows * total_cols];

    let mut spreadsheet = vec![vec![Cell { 
        value: CellValue::Int(0), dependents: HashSet::new(), formula: None }; total_cols]; total_rows];

    let (mut start_row, mut start_col) = (0, 0);
    let mut enable_output = true;

    let prompt = |elapsed: f64, status: &str| {
        print!("[{:.1}] ({}) > ", elapsed, status);
        io::stdout().flush().unwrap();
    };

    let start_time = Instant::now();
    printsheet(&spreadsheet, start_row, start_col, total_rows, total_cols);
    prompt(start_time.elapsed().as_secs_f64(), STATUS[unsafe { STATUS_CODE }]);

    loop {
        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input).unwrap();
        if bytes_read == 0 {break; }
        println!();
        let start_time = Instant::now();
        let input = input.trim();
        unsafe { STATUS_CODE = 0; }

        match input {
            "w" => w(&mut start_row),
            "s" => s(&mut start_row, total_rows),
            "a" => a(&mut start_col),
            "d" => d(&mut start_col, total_cols),
            "q" => break,
            _ if input.contains('=') => {
                let parts: Vec<&str> = input.splitn(2, '=').map(str::trim).collect();
                if parts.len() == 2 {
                    let (cell_ref, formula) = (parts[0], parts[1]);
                    let (row, col) = to_indices(cell_ref);
                    if row < total_rows && col < total_cols {
                        update_cell(&mut spreadsheet, total_rows, total_cols, row, col, formula, &mut visited);
                        if unsafe { STATUS_CODE } == 0 {
                            recalc(&mut spreadsheet, total_rows, total_cols, row, col);
                        }
                    } else {
                        unsafe { STATUS_CODE = 1; }
                    }
                }
            }
            _ if input.starts_with("scroll_to ") => {
                let cell_ref = input.trim_start_matches("scroll_to ").trim();
                if cell_ref.is_empty()
                    || !cell_ref.chars().next().unwrap().is_alphabetic()
                    || scroll_to(&mut start_row, &mut start_col, total_rows, total_cols, cell_ref).is_err()
                {
                    unsafe { STATUS_CODE = 1; }
                }
            }
            "disable_output" => enable_output = false,
            "enable_output" => enable_output = true,
            _ => unsafe { STATUS_CODE = 2; },
        }
        if enable_output {
            printsheet(&spreadsheet, start_row, start_col, total_rows, total_cols);
        }
        prompt(start_time.elapsed().as_secs_f64(), STATUS[unsafe { STATUS_CODE }]);
    }
}