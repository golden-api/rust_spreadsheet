use std::collections::HashSet;

use eframe::egui::{
    Color32,
    Stroke,
    Vec2,
};

use crate::{
    Cell,
    CellData,
    Valtype,
};
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

// Define your styling configuration.
pub struct SpreadsheetStyle {
    pub(crate) header_bg:          Color32,
    pub(crate) header_text:        Color32,
    pub(crate) cell_bg_even:       Color32,
    pub(crate) cell_bg_odd:        Color32,
    pub(crate) cell_text:          Color32,
    pub(crate) selected_cell_bg:   Color32,
    pub(crate) selected_cell_text: Color32,
    pub(crate) grid_line:          Stroke,
    pub(crate) cell_size:          Vec2,
    pub(crate) font_size:          f32,
    pub(crate) prev_base_color:    Color32,
    pub(crate) rainbow:            u32,
    pub(crate) frequency:          f32,
    // pub(crate) frequency1:          f32,
    pub(crate) matrix_raindrops:   Vec<(usize, usize, f32, usize)>, // (column, row, speed, length)
    pub(crate) get_cell_bg: Option<Box<dyn Fn(usize, usize) -> Color32>>, // Function to get cell background
}

impl Default for SpreadsheetStyle {
    fn default() -> Self {
        Self {
            header_bg:          Color32::from_rgb(60, 63, 100),
            header_text:        Color32::from_rgb(220, 220, 220),
            cell_bg_even:       Color32::from_rgb(65, 50, 85),
            cell_bg_odd:        Color32::from_rgb(45, 45, 45),
            cell_text:          Color32::LIGHT_GRAY,
            selected_cell_bg:   Color32::from_rgb(120, 120, 180),
            selected_cell_text: Color32::WHITE,
            grid_line:          Stroke::new(1.0, Color32::from_rgb(70, 70, 70)),
            cell_size:          Vec2::new(60.0, 25.0),
            font_size:          14.0,
            prev_base_color:    Color32::from_rgb(120, 120, 180),
            rainbow:            0,
            frequency:          0.2,
            // frequency1:          0.0,
            matrix_raindrops:   Vec::new(),
            get_cell_bg:        None,
        }
    }
}

pub struct SpreadsheetApp {
    pub(crate) sheet:                 Vec<Vec<Cell>>,
    pub(crate) selected:              Option<(usize, usize)>,
    pub(crate) formula_input:         String,
    pub(crate) editing_cell:          bool,
    pub(crate) style:                 SpreadsheetStyle,
    pub(crate) status_message:        String,
    pub(crate) start_row:             usize,
    pub(crate) start_col:             usize,
    pub(crate) scroll_to_cell:        String,
    pub(crate) should_reset_scroll:   bool,
    pub(crate) focus_on:              usize,
    pub(crate) request_formula_focus: bool,
    pub(crate) clipboard:             Option<Cell>,
    pub(crate) clipboard_formula:     String,
    pub(crate) undo_stack:            Vec<UndoAction>,
    pub(crate) redo_stack:            Vec<UndoAction>,
    pub(crate) max_undo_levels:       usize,
    pub(crate) show_save_dialog:      bool,
    pub(crate) save_filename:         String,
}

impl SpreadsheetApp {
    pub fn new(
        rows: usize,
        cols: usize,
        start_row: usize,
        start_col: usize,
    ) -> Self {
        let sheet = vec![vec![Cell { value: Valtype::Int(0), data: CellData::Empty, dependents: HashSet::new() }; cols]; rows];
        Self {
            sheet,
            selected: Some((0, 0)),
            formula_input: String::new(),
            editing_cell: false,
            style: SpreadsheetStyle::default(),
            status_message: String::new(),
            start_row,
            start_col,
            scroll_to_cell: String::new(),
            should_reset_scroll: false,
            focus_on: 0,
            request_formula_focus: false,
            clipboard: None,
            clipboard_formula: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_levels: 100,
            show_save_dialog: false,
            save_filename: String::new(),
        }
    }
}

pub struct UndoAction {
    pub position:    (usize, usize), // (row, col)
    pub old_cell:    Cell,
    pub old_formula: String,
}