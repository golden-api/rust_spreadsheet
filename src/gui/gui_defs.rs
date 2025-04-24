use eframe::egui::{Color32, Stroke, Vec2};

use crate::Cell;
use crate::HashMap;

/// Represents the direction of movement or scrolling in the spreadsheet interface.
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Defines the styling configuration for the spreadsheet GUI.
///
/// # Fields
/// * `header_bg` - Background color for header cells.
/// * `header_text` - Text color for header cells.
/// * `cell_bg_even` - Background color for even-numbered cells.
/// * `cell_bg_odd` - Background color for odd-numbered cells.
/// * `cell_text` - Text color for regular cells.
/// * `selected_cell_bg` - Background color for the selected cell.
/// * `selected_cell_text` - Text color for the selected cell.
/// * `grid_line` - Stroke style for grid lines.
/// * `cell_size` - Size of each cell as a 2D vector.
/// * `font_size` - Font size for text in cells.
/// * `prev_base_color` - Previous base color for animations or transitions.
/// * `rainbow` - Counter for rainbow animation effect.
/// * `frequency` - Frequency of the rainbow animation effect.
/// * `matrix_raindrops` - Vector of raindrop effects for matrix-style visuals.
/// * `get_cell_bg` - Optional function to dynamically determine cell background color.
/// * `range_selection_bg` - Background color for range selection.
/// * `range_selection_text` - Text color for range selection.
pub struct SpreadsheetStyle {
    pub(crate) header_bg: Color32,
    pub(crate) header_text: Color32,
    pub(crate) cell_bg_even: Color32,
    pub(crate) cell_bg_odd: Color32,
    pub(crate) cell_text: Color32,
    pub(crate) selected_cell_bg: Color32,
    pub(crate) selected_cell_text: Color32,
    pub(crate) grid_line: Stroke,
    pub(crate) cell_size: Vec2,
    pub(crate) font_size: f32,
    pub(crate) prev_base_color: Color32,
    pub(crate) rainbow: u32,
    pub(crate) frequency: f32,
    pub(crate) matrix_raindrops: Vec<(usize, usize, f32, usize)>, // (column, row, speed, length)
    pub(crate) get_cell_bg: Option<Box<dyn Fn(usize, usize) -> Color32>>, // Function to get cell background
    pub(crate) range_selection_bg: Color32,
    pub(crate) range_selection_text: Color32,
}

impl Default for SpreadsheetStyle {
    /// Creates a default `SpreadsheetStyle` with predefined colors and settings.
    ///
    /// # Returns
    /// A `SpreadsheetStyle` instance with default values.
    fn default() -> Self {
        Self {
            header_bg: Color32::from_rgb(60, 63, 100),
            header_text: Color32::from_rgb(220, 220, 220),
            cell_bg_even: Color32::from_rgb(65, 50, 85),
            cell_bg_odd: Color32::from_rgb(45, 45, 45),
            cell_text: Color32::LIGHT_GRAY,
            selected_cell_bg: Color32::from_rgb(120, 120, 180),
            selected_cell_text: Color32::WHITE,
            grid_line: Stroke::new(1.0, Color32::from_rgb(70, 70, 70)),
            cell_size: Vec2::new(60.0, 25.0),
            font_size: 14.0,
            prev_base_color: Color32::from_rgb(120, 120, 180),
            rainbow: 0,
            frequency: 0.2,
            matrix_raindrops: Vec::new(),
            get_cell_bg: None,
            range_selection_bg: Color32::from_rgb(80, 160, 160), // Lighter blue
            range_selection_text: Color32::from_rgb(230, 230, 230),
        }
    }
}

/// Represents the state and configuration of the spreadsheet application in GUI mode.
///
/// # Fields
/// * `sheet` - Hash map storing cell data.
/// * `ranged` - Hash map tracking range dependencies.
/// * `is_range` - Boolean vector indicating range membership.
/// * `total_rows` - Total number of rows.
/// * `total_cols` - Total number of columns.
/// * `selected` - Optional tuple of the currently selected cell (row, col).
/// * `formula_input` - String for the current formula input.
/// * `editing_cell` - Boolean indicating if a cell is being edited.
/// * `style` - Styling configuration for the GUI.
/// * `status_message` - Current status message to display.
/// * `start_row` - Starting row index for the visible area.
/// * `start_col` - Starting column index for the visible area.
/// * `scroll_to_cell` - String for the cell to scroll to.
/// * `should_reset_scroll` - Boolean to trigger scroll reset.
/// * `focus_on` - Index for focusing on a specific element.
/// * `request_formula_focus` - Boolean to request focus on formula input.
/// * `clipboard` - Optional cell data for clipboard.
/// * `clipboard_formula` - Formula string in the clipboard.
/// * `undo_stack` - Stack of undo actions.
/// * `redo_stack` - Stack of redo actions.
/// * `max_undo_levels` - Maximum number of undo levels.
/// * `show_save_dialog` - Boolean to show the save dialog.
/// * `save_filename` - Filename for saving the spreadsheet.
/// * `range_start` - Optional starting point of a range selection.
/// * `range_end` - Optional ending point of a range selection.
/// * `is_selecting_range` - Boolean indicating range selection mode.
pub struct SpreadsheetApp {
    pub(crate) sheet: HashMap<u32, Cell>,
    pub(crate) ranged: HashMap<u32, Vec<(u32, u32)>>,
    pub(crate) is_range: Vec<bool>,
    pub(crate) total_rows: usize,
    pub(crate) total_cols: usize,
    pub(crate) selected: Option<(usize, usize)>,
    pub(crate) formula_input: String,
    pub(crate) editing_cell: bool,
    pub(crate) style: SpreadsheetStyle,
    pub(crate) status_message: String,
    pub(crate) start_row: usize,
    pub(crate) start_col: usize,
    pub(crate) scroll_to_cell: String,
    pub(crate) should_reset_scroll: bool,
    pub(crate) focus_on: usize,
    pub(crate) request_formula_focus: bool,
    pub(crate) clipboard: Option<Cell>,
    pub(crate) clipboard_formula: String,
    pub(crate) undo_stack: Vec<UndoAction>,
    pub(crate) redo_stack: Vec<UndoAction>,
    pub(crate) max_undo_levels: usize,
    pub(crate) show_save_dialog: bool,
    pub(crate) save_filename: String,
    pub(crate) range_start: Option<(usize, usize)>,
    pub(crate) range_end: Option<(usize, usize)>,
    pub(crate) is_selecting_range: bool,
}

impl SpreadsheetApp {
    /// Creates a new `SpreadsheetApp` instance with the specified dimensions.
    ///
    /// # Arguments
    /// * `rows` - The total number of rows.
    /// * `cols` - The total number of columns.
    /// * `start_row` - The initial starting row index.
    /// * `start_col` - The initial starting column index.
    ///
    /// # Returns
    /// A `SpreadsheetApp` instance initialized with default values.
    pub fn new(rows: usize, cols: usize, start_row: usize, start_col: usize) -> Self {
        let sheet: HashMap<u32, Cell> = HashMap::with_capacity(1024);
        let ranged: HashMap<u32, Vec<(u32, u32)>> = HashMap::with_capacity(512);
        let is_range: Vec<bool> = vec![false; rows * cols];
        let total_rows = rows;
        let total_cols = cols;
        Self {
            sheet,
            ranged,
            is_range,
            total_rows,
            total_cols,
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
            range_start: None,
            range_end: None,
            is_selecting_range: false,
        }
    }
}

/// Represents an action to undo or redo in the spreadsheet.
///
/// # Fields
/// * `position` - Tuple of (row, col) indicating the cell position.
/// * `old_cell` - The previous state of the cell.
/// * `old_formula` - The previous formula associated with the cell.
pub struct UndoAction {
    pub position: (usize, usize), // (row, col)
    pub old_cell: Cell,
    pub old_formula: String,
}
