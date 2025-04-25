# Rust Spreadsheet Application

A spreadsheet application written in Rust, featuring both a command-line interface and a graphical user interface (GUI). It supports basic spreadsheet functionalities, including cell formulas, range operations, and dependency management, making it suitable for educational or lightweight spreadsheet tasks.

## Features

### Command-Line Mode
- **Interactive Interface**: Enter commands to manipulate the spreadsheet in real-time.
- **Formula Support**: Handles arithmetic operations (e.g., `A1 = B1 + 10`) and cell references.
- **Range Functions**: Supports SUM, AVG, MAX, MIN, and STDEV for cell ranges.
- **Navigation Commands**: Use `w`, `s`, `a`, `d` to scroll the view, or `scroll_to <cell>` to jump to a specific cell.
- **Dependency Tracking**: Automatically updates dependent cells with cycle detection to prevent infinite loops.
- **Output Control**: Toggle spreadsheet display with `disable_output` and `enable_output` commands.

### GUI Mode
- **Graphical Interface**: Built with [eframe](https://github.com/emilk/egui/tree/master/crates/eframe) and [egui](https://github.com/emilk/egui) for a modern, responsive UI.
- **Cell Interaction**: Select and edit cells using mouse or keyboard input.
- **Formula Input Bar**: Enter and edit formulas directly in a dedicated bar.
- **Range Selection**: Highlight and operate on cell ranges for functions like SUM or AVG.
- **Navigation**: Scroll through the spreadsheet with mouse or keyboard shortcuts.
- **Clipboard Support**: Copy and paste cell data for efficient editing.
- **Undo/Redo**: Revert or reapply changes to maintain workflow flexibility.
- **Customizable Themes**: Apply visual styles, including animations like rainbow effects or Matrix-style raindrops.
- **File Operations**: Save and load spreadsheets for persistent data management.

## Installation

1. **Install Rust**: Ensure you have Rust installed. Follow the [official installation guide](https://www.rust-lang.org/tools/install) if needed.
2. **Clone the Repository**:
   ```bash
   git clone <repository_url>
   cd <repository_name>
   ```
3. **Build the Project**:
   - For command-line mode:
     ```bash
     cargo build --release --features autograder
     ```
   - For GUI mode:
     ```bash
     cargo build --release --features gui
     ```

## Usage

### Command-Line Mode
Run the application with specified dimensions (rows: 1–999, columns: 1–18,278):
```bash
cargo run --release --features autograder -- <rows> <cols>
```
**Example**:
```bash
cargo run --release --features autograder -- 10 10
```
**Commands**:
- Set a cell value: `A1 = 5`
- Use formulas: `B1 = A1 + 3`
- Navigate: `w` (up), `s` (down), `a` (left), `d` (right)
- Jump to a cell: `scroll_to A1`
- Quit: `q`
- Toggle output: `disable_output` or `enable_output`

The application displays a 10x10 grid of the spreadsheet, with column headers (e.g., A, B) and row numbers, updating after each command with status messages (e.g., "ok", "cycle detected").

### GUI Mode
Run the application with specified dimensions:
```bash
cargo run --release --features gui -- <rows> <cols>
```
**Example**:
```bash
cargo run --release --features gui -- 10 10
```
**Interaction**:
- Click to select cells or Right Click on first and last to select the range between them.
- Enter formulas in the top input bar (e.g., `=A1+5` in the command mode else just write A1+5 and enter) , also double click a cell to enter edit cell mode similar can be done if I press just space key on a cell.
- Use keyboard shortcuts (arrow keys for navigation, Ctrl+E/Ctrl+T/Ctrl+R for copy/cut/paste,space key on a cell to enter edit-cell mode,Esc key to switch to command mode or cancel a formula).
- Save spreadsheets in csv file like an excel sheet with constraint (fcsv <filename>) or like a plain csv file(csv <filename>) in the command mode .
- Apply themes or animations through style settings(by themes button you may select or also change the pre-defined themes check from help command in cmd mode)
- goto and scroll_to feature also there(a separate scroll_to button also there) goto moves your selection to the specified cell and scroll to takes the screen to there .
- multi_selection also supported with minimal operations like if I have selected a range of cells and then in the single selected cell I enter formula like MAX() and enter then the range is automaticallly taken in .
- scrolling can be done by mouse also, with scroll bars too, also in command mode by w<Option(number)>,s,a,d similarly .  

Alternatively, use the Makefile target for GUI mode with maximum dimensions:
```bash
make ext1
```

## Architecture

The application is modular, separating core logic from user interfaces:
- **Core Logic**: Manages spreadsheet data, formula parsing, evaluation, and dependency tracking. Key modules include `parser.rs` for formula handling and `utils.rs` for general utilities.
- **Command-Line Interface**: Provides a text-based, interactive frontend for direct command input.
- **Graphical User Interface**: Leverages eframe and egui for a visual frontend, with modules like `gui_defs.rs` and `render_gui.rs` handling state and rendering.

This design ensures maintainability and allows potential extensions, such as adding new formula types or UI features.

## Design Decisions

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| **Cell Representation** | Uses a `CellName` struct with a 7-byte limit and ASCII-only characters. | Optimizes memory usage and simplifies parsing, though restricts cell name complexity. |
| **Spreadsheet Size** | Limits to 999 rows and 18,278 columns. | Balances performance and memory constraints, suitable for most educational use cases. |
| **Command-Line View** | Displays a 10x10 grid at a time. | Keeps output readable and manageable on terminal screens. |
| **GUI Rendering** | Caps visible rows at 33 and rendering at 300 columns/500 rows. | Prevents performance degradation with large spreadsheets, though requires scrolling. |
| **Formula Parsing** | Custom parser with regex for arithmetic and range functions. | Provides flexibility and control, supporting essential spreadsheet operations. |
| **Feature Flags** | Uses `autograder` for command-line and `gui` for GUI mode. | Enables conditional compilation, reducing binary size and allowing mode-specific builds. |
| **Dependency Management** | Employs HashMap for cells and HashSet for dependents, with topological sorting. | Ensures efficient data access and cycle detection, critical for dynamic updates. |

## Limitations

| Limitation | Description | Impact |
|------------|-------------|--------|
| **Cell Name Length** | Limited to 7 characters, ASCII-only. | Prevents use of longer or non-ASCII names, potentially limiting expressiveness. |
| **Spreadsheet Size** | Maximum 999 rows, 18,278 columns. | Restricts scalability for very large datasets. |
| **Command-Line View** | Shows only 10x10 cells at a time. | May require frequent scrolling for larger spreadsheets, reducing visibility. |
| **GUI Performance** | Slowdowns possible with large spreadsheets due to rendering caps. | Affects usability for complex or large-scale applications. |
| **Function Support** | Limited to basic arithmetic and range functions (SUM, AVG, MAX, MIN, STDEV). | Lacks advanced features like charting, macros, or complex statistical functions found in commercial software. |
| **Command Processing** | Unknown commands in GUI mode result in error messages. | Requires precise input, potentially necessitating better documentation or error handling. |

## Challenges

| Challenge | Description | Solution |
|-----------|-------------|----------|
| **Dependency Management** | Ensuring acyclic dependency graphs and efficient updates. | Uses topological sorting (Kahn’s algorithm) and BFS for recalculation, with cycle detection to rollback changes. |
| **Performance** | Handling large spreadsheets without slowdowns, especially in GUI mode. | Implements rendering caps and sparse data structures (HashMap) to optimize memory and computation. |
| **User Interface** | Providing intuitive interaction in both modes. | Command-line uses simple commands and status feedback; GUI offers mouse/keyboard input and visual cues. |
| **Formula Evaluation** | Parsing and evaluating complex formulas accurately. | Custom parser with regex and robust error handling, though limited to predefined function types. |
| **Thread Safety** | Managing global variables like `STATUS_CODE` in command-line mode. | Uses `unsafe` Rust code with careful access, though future improvements could explore safer alternatives. |

## Development

The project includes a Makefile for streamlined development tasks:
- `make build`: Builds with the `autograder` feature for command-line mode.
- `make test`: Runs tests with `autograder` feature, single-threaded.
- `make coverage`: Generates test coverage using [cargo-tarpaulin](https://crates.io/crates/cargo-tarpaulin).
- `make docs`: Generates and opens documentation with all features enabled.
- `make clean`: Removes build artifacts.

To contribute:
1. Fork the repository.
2. Create a feature branch (`git checkout -b feature-name`).
3. Commit changes (`git commit -m "Add feature"`).
4. Push to the branch (`git push origin feature-name`).
5. Open a pull request.

## License

[Specify the license or link to the LICENSE file]
