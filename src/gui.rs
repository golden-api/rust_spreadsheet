use iced::{
    widget::{Button, Column, Container, Row, Text, TextInput, Scrollable},
    Element, Length, Renderer, Application, Command, Theme, Settings
};
use iced::executor;

use crate::{Cell, CellValue};
use crate::utils::update_cell;
use crate::parser::detect_formula;

pub fn run_gui(num_rows: usize, num_cols: usize) {
    let settings = Settings {
        flags: (num_rows, num_cols), // Pass the row/col tuple here
        window: iced::window::Settings {
            size: (800, 600),
            ..Default::default()
        },
        // You can configure more settings here as needed
        ..Default::default()
    };

    // Now pass these settings into `Spreadsheet::run`
    let _ = Spreadsheet::run(settings);
}

#[derive(Debug, Clone)]
pub enum Message {
    FormulaChanged(String),
    UpdateCell,
    CellSelected(usize, usize),
}

pub struct Spreadsheet {
    sheet: Vec<Vec<Cell>>,
    selected: (usize, usize),
    formula_input: String,
}

impl Application for Spreadsheet {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = (usize, usize); // (num_rows, num_cols)
    type Theme = Theme;

    fn new(flags: Self::Flags) -> (Self, Command<Message>){
        let (rows, cols) = flags;
        let sheet = vec![vec![Cell::default(); cols]; rows];
        (
            Spreadsheet {
                sheet,
                selected: (0, 0),
                formula_input: String::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Rust Spreadsheet")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::FormulaChanged(new_formula) => {
                self.formula_input = new_formula;
            }
            Message::UpdateCell => {
                let (r, c) = self.selected;
                let total_rows = self.sheet.len();
                let total_cols = if total_rows > 0 { self.sheet[0].len() } else { 0 };
                // Create a visited vector for cycle detection
                let mut visited = vec![0u8; total_rows * total_cols];
                update_cell(&mut self.sheet, total_rows, total_cols, r, c, &self.formula_input, &mut visited);
            }
            Message::CellSelected(r, c) => {
                self.selected = (r, c);
                // Show the current formula of the selected cell, if any.
                self.formula_input = self.sheet[r][c].formula.clone().unwrap_or_default();
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        // Build the formula bar
        let formula_bar = TextInput::new(
            "Enter formula...",
            &self.formula_input,
            Message::FormulaChanged,
        )
        .padding(10);
        
        let update_btn = Button::new(Text::new("Update Cell"))
            .on_press(Message::UpdateCell)
            .padding(10);

        // Build the grid view as a scrollable column
        let mut grid = Column::new().spacing(5);
        for (i, row) in self.sheet.iter().enumerate() {
            let mut row_view = Row::new().spacing(5);
            for (j, cell) in row.iter().enumerate() {
                let display_text = match &cell.value {
                    CellValue::Int(v) => v.to_string(),
                    CellValue::Str(s) => s.clone(),
                };
                let cell_button = Button::new(Text::new(display_text))
                    .on_press(Message::CellSelected(i, j))
                    .padding(5)
                    .width(Length::Fixed(80.0))
                    .height(Length::Fixed(30.0));
                row_view = row_view.push(cell_button);
            }
            grid = grid.push(row_view);
        }
        
        let scrollable_grid = Scrollable::new(grid);

        let content = Column::new()
            .spacing(20)
            .push(Row::new().spacing(10).push(formula_bar).push(update_btn))
            .push(scrollable_grid);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}
