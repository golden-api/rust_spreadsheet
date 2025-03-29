use std::{collections::HashSet, env, io::{self, Write}, process, time::Instant};
mod scrolling;
mod utils;
mod parser;
mod dependency;
const STATUS: [&str; 4] = ["ok", "Invalid range", "unrecognized cmd", "cycle detected"];
pub static mut STATUS_CODE:usize=0;

#[derive(Clone)]
pub enum FormulaType {SleepC,SleepR,Const,Ref,CoR,RoC,CoC,RoR,Range,Invalid}
#[derive(Clone)]
pub enum Valtype {Int(i32),Str(String)}

#[derive(Clone)]
pub struct Cell {
    value: Valtype,formula :Option<FormulaType>, value2 : Valtype,
    op_code : Option<char>,
    cell1 : Option<String>,cell2 : Option<String>,
    dependents: HashSet<(usize, usize)>
}
impl Cell {
    pub fn reset(&mut self) {
        let current_dependents = std::mem::take(&mut self.dependents);
        *self = Self {
            value: Valtype::Int(0),value2: Valtype::Int(0),
            formula: None,op_code: None,cell1: None,cell2: None,
            dependents: current_dependents,
        };}

    pub fn my_clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            value2: self.value2.clone(),
            formula: self.formula.clone(),
            op_code: self.op_code.clone(),
            cell1: self.cell1.clone(),
            cell2: self.cell2.clone(),
            dependents: HashSet::new(),
            }
        }
}

fn print_sheet(spreadsheet: &[Vec<Cell>], pointer: &(usize, usize), dimension: &(usize, usize)) {
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
                    Valtype::Str(s) => print!("{:<10}  ", s),
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
    let (total_rows, total_cols) = match parse_dimensions(args) {
        Ok(dim) => dim,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };
    let mut visited = vec![0u8; (total_rows  )* (total_cols )];

    let mut spreadsheet= vec![vec![Cell { 
        value: Valtype::Int(0),value2:Valtype::Int(0),dependents: HashSet::new(), formula: None,op_code: None,cell1: None,cell2: None
    };total_cols]; total_rows];
    let (mut start_row, mut start_col) = (0,0);
    let mut enable_output = true;

    let prompt = |elapsed: f64, status: &str| {
        print!("[{:.1}] ({}) > ", elapsed, status);
        io::stdout().flush().unwrap();
    };

    let start_time = Instant::now();
    print_sheet(&spreadsheet, &(start_row, start_col), &(total_rows, total_cols));
    prompt(start_time.elapsed().as_secs_f64(), STATUS[unsafe {STATUS_CODE}]);

    loop {
        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input).unwrap();
        if bytes_read == 0 {break; }
        println!();
        let start_time = Instant::now();
        let input = input.trim();
        unsafe { STATUS_CODE = 0; }
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
                    if row < total_rows && col < total_cols {
                        let old_cell=spreadsheet[row][col].my_clone();
                        parser::detect_formula(&mut spreadsheet[row][col],formula);
                        dependency::update_cell(&mut spreadsheet, total_rows, total_cols, row, col, &mut visited,old_cell);
                        if unsafe { STATUS_CODE } == 0 {
                            parser::recalc(&mut spreadsheet, total_rows, total_cols, row, col);
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
                    || scrolling::scroll_to(&mut start_row, &mut start_col, total_rows, total_cols, cell_ref).is_err()
                {
                    unsafe { STATUS_CODE = 1; }
                }
            }
            "disable_output" => enable_output = false,
            "enable_output" => enable_output = true,
            _ => unsafe { STATUS_CODE = 2; },
        }
        if enable_output {
            print_sheet(&spreadsheet, &(start_row, start_col), &(total_rows, total_cols));
        }
        prompt(start_time.elapsed().as_secs_f64(), STATUS[unsafe { STATUS_CODE }]);
    }
}