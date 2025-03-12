mod utils;
mod scrolling;
// mod parser;
use utils::to_indices;
use scrolling::*;
// use parser::*;
use std::collections::BTreeMap;
use std::env;
use std::io::{self, Write};
use std::process;
use std::time::Instant;

const MAX_ROWS: usize = 10;
const MAX_COLS: usize = 10;
const STATUS: [&str; 4] = ["ok", "Invalid range", "unrecognized cmd", "cycle detected"];

#[derive(Clone)]
enum CellValue {
    Int(i32),
    Str(String),
}

impl Default for CellValue {
    fn default() -> Self {CellValue::Int(0) }
}

#[derive(Clone, Default)]
struct Cell {
    value: CellValue,
    formula: Option<String>,
    dependents: BTreeMap<(usize, usize), ()>,
}

fn printsheet(spreadsheet: &Vec<Vec<Cell>>, start_row: usize, start_col: usize, total_rows: usize, total_cols: usize) {
    let col_name = |col: usize| -> String {
        let mut n = col + 1;
        let mut temp = Vec::new();
        while n > 0 {
            let rem = (n - 1) % 26;
            temp.push((b'A' + rem as u8) as char);
            n = (n - 1) / 26;
        }
        temp.reverse();
        temp.into_iter().collect()
    };

    let view_rows = total_rows.saturating_sub(start_row).min(MAX_ROWS);
    let view_cols = total_cols.saturating_sub(start_col).min(MAX_COLS);

    print!("{:<5}", "");
    for j in 0..view_cols {
        print!("{:>10}  ", col_name(start_col + j));
    }
    println!();

    for i in 0..view_rows {
        print!("{:4}  ", start_row + i + 1);
        for j in 0..view_cols {
            let cell = &spreadsheet[start_row + i][start_col + j];
            match &cell.value {
                CellValue::Int(v) => print!("{:<10}  ", v),
                CellValue::Str(s) => print!("{:<10}  ", s),
            }
        }
        println!();
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <num_rows> <num_columns>", args[0]);
        process::exit(1);
    }

    let total_rows = args[1].parse::<usize>().unwrap_or(0);
    let total_cols = args[2].parse::<usize>().unwrap_or(0);
    if total_rows < 1 || total_rows > 999 || total_cols < 1 || total_cols > 18278 {
        eprintln!("Invalid dimensions.");
        process::exit(1);
    }

    let mut spreadsheet = vec![vec![Cell::default(); total_cols]; total_rows];
    let (mut start_row, mut start_col, mut status_code) = (0, 0, 0);
    let mut enable_output = true;
    let mut start_time = Instant::now();

    printsheet(&spreadsheet, start_row, start_col, total_rows, total_cols);
    print!("[{:.1}] ({}) > ", start_time.elapsed().as_secs_f64(), STATUS[status_code]);
    io::stdout().flush().unwrap();

    loop {
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }
        println!();
        start_time = Instant::now();
        let input = input.trim();
        status_code = 0;

        match input {
            "W" => w(&mut start_row),
            "S" => s(&mut start_row, total_rows),
            "A" => a(&mut start_col),
            "D" => d(&mut start_col, total_cols),
            "q" => break,
            // _ if input.contains('=') => {
            //     let parts: Vec<&str> = input.splitn(2, '=').collect();
            //     if let [cell_ref, formula] = &parts[..] {
            //         let (row, col) = to_indices(cell_ref.trim());
            //         if row < total_rows && col < total_cols {
            //             if update_cell(&mut spreadsheet, row, col, formula.trim()).is_ok() {
            //                 recalc(&mut spreadsheet, row, col);
            //             }
            //         } else {
            //             status_code = 1;
            //         }
            //     }
            // }
            _ if input.starts_with("scroll_to ") => {
                let cell_ref = input.trim_start_matches("scroll_to ").trim();
                if cell_ref.is_empty() || !cell_ref.chars().next().unwrap().is_alphabetic() {
                    status_code = 1;
                } else if scroll_to(&mut start_row, &mut start_col, total_rows, total_cols, cell_ref).is_err() {
                    status_code = 1;
                }
            }
            "disable_output" => enable_output = false,
            "enable_output" => enable_output = true,
            _ => status_code = 2,
        }

        if enable_output {
            printsheet(&spreadsheet, start_row, start_col, total_rows, total_cols);
        }
        print!("[{:.1}] ({}) > ", start_time.elapsed().as_secs_f64(), STATUS[status_code]);
        io::stdout().flush().unwrap();
    }
}