use std::collections::{HashMap, HashSet, VecDeque};
use crate::{Cell, FormulaType, utils::to_indices, STATUS_CODE};

/// Remove a dependency link from a dependent cell.
pub fn remove_reference(dep_cell: &mut Cell, target: (usize, usize)) {
    dep_cell.dependents.remove(&target);
}

/// For range formulas, add dependencies from every cell in the specified range to cell (r, c).
pub fn add_range_dependencies(
    sheet: &mut Vec<Vec<Cell>>,
    start_ref: &str,
    end_ref: &str,
    r: usize,
    c: usize,
) {
    let (start_row, start_col) = to_indices(start_ref);
    let (end_row, end_col) = to_indices(end_ref);
    for row in start_row..=end_row {
        for col in start_col..=end_col {
            sheet[row][col].dependents.insert((r, c));
        }
    }
}

/// Detect cycle using Tarjan’s Algorithm.
/// First, we perform a BFS from the starting cell to build the affected subgraph,
/// then run Tarjan’s algorithm on that subgraph.
/// Returns true if a cycle is detected.
pub fn detect_cycle(
    sheet: &Vec<Vec<Cell>>,
    start: (usize, usize),
    total_rows: usize,
    total_cols: usize,
) -> bool {
    // Phase 1: Build the affected subgraph using BFS.
    let mut affected = Vec::new();
    let mut visited = HashSet::new();
    let mut bfs = VecDeque::new();
    bfs.push_back(start);
    visited.insert(start);

    while let Some(pos) = bfs.pop_front() {
        affected.push(pos);
        let cell = &sheet[pos.0][pos.1];
        for &(dep_row, dep_col) in &cell.dependents {
            let neighbor = (dep_row, dep_col);
            if neighbor.0 < total_rows && neighbor.1 < total_cols && !visited.contains(&neighbor) {
                visited.insert(neighbor);
                bfs.push_back(neighbor);
            }
        }
    }

    // Phase 2: Build a graph (adjacency list) from the affected nodes.
    let mut graph: HashMap<(usize, usize), Vec<(usize, usize)>> = HashMap::new();
    for &node in &affected {
        let cell = &sheet[node.0][node.1];
        let mut neighbors = Vec::new();
        for &(dep_row, dep_col) in &cell.dependents {
            let neighbor = (dep_row, dep_col);
            if visited.contains(&neighbor) {
                neighbors.push(neighbor);
            }
        }
        graph.insert(node, neighbors);
    }

    // Phase 3: Run Tarjan’s Algorithm on the affected subgraph.
    let mut index_map: HashMap<(usize, usize), usize> = HashMap::new();
    let mut lowlink: HashMap<(usize, usize), usize> = HashMap::new();
    let mut index = 0;
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let mut on_stack: HashSet<(usize, usize)> = HashSet::new();
    let mut cycle_found = false;

    fn strongconnect(
        node: (usize, usize),
        graph: &HashMap<(usize, usize), Vec<(usize, usize)>>,
        index_map: &mut HashMap<(usize, usize), usize>,
        lowlink: &mut HashMap<(usize, usize), usize>,
        index: &mut usize,
        stack: &mut Vec<(usize, usize)>,
        on_stack: &mut HashSet<(usize, usize)>,
        cycle_found: &mut bool,
    ) {
        index_map.insert(node, *index);
        lowlink.insert(node, *index);
        *index += 1;
        stack.push(node);
        on_stack.insert(node);

        if let Some(neighbors) = graph.get(&node) {
            for &neighbor in neighbors {
                if !index_map.contains_key(&neighbor) {
                    strongconnect(neighbor, graph, index_map, lowlink, index, stack, on_stack, cycle_found);
                    let node_low = lowlink[&node];
                    let neigh_low = lowlink[&neighbor];
                    lowlink.insert(node, node_low.min(neigh_low));
                } else if on_stack.contains(&neighbor) {
                    let node_low = lowlink[&node];
                    let neigh_index = index_map[&neighbor];
                    lowlink.insert(node, node_low.min(neigh_index));
                }
            }
        }

        // If node is a root node, pop the stack and generate an SCC.
        if lowlink[&node] == index_map[&node] {
            let mut scc = Vec::new();
            loop {
                let w = stack.pop().unwrap();
                on_stack.remove(&w);
                scc.push(w);
                if w == node {
                    break;
                }
            }
            // If SCC has more than one node or a self-loop, a cycle exists.
            if scc.len() > 1 || (scc.len() == 1 && graph.get(&node).unwrap_or(&vec![]).contains(&node)) {
                *cycle_found = true;
            }
        }
    }

    for &node in &affected {
        if !index_map.contains_key(&node) {
            strongconnect(
                node,
                &graph,
                &mut index_map,
                &mut lowlink,
                &mut index,
                &mut stack,
                &mut on_stack,
                &mut cycle_found,
            );
        }
        if cycle_found {
            break;
        }
    }

    cycle_found
}

/// Update the cell at (r, c) in the sheet with the new formula information.
/// This function performs:
/// 1. Validation of new cell references (skipping detailed checks here).
/// 2. Cycle detection using the Tarjan-based algorithm.
/// 3. Removal of old dependency links from backup.
/// 4. Addition of new dependency links.
/// 5. (Assumes recalculation is handled separately.)
pub fn update_cell(
    sheet: &mut Vec<Vec<Cell>>,
    total_rows: usize,
    total_cols: usize,
    r: usize,
    c: usize,
    mut backup: Cell,
) {
    // Basic validation of new references.
    {
        match &sheet[r][c].formula {
            Some(FormulaType::Invalid) => {
                unsafe { STATUS_CODE = 2; }
                return;
            }
            _ => {
                if let Some(ref r1) = sheet[r][c].cell1 {
                    let (row_idx, col_idx) = to_indices(r1);
                    if row_idx >= total_rows || col_idx >= total_cols {
                        unsafe { STATUS_CODE = 1; }
                    }
                }
                if let Some(ref r2) = sheet[r][c].cell2 {
                    let (row_idx, col_idx) = to_indices(r2);
                    if row_idx >= total_rows || col_idx >= total_cols {
                        unsafe { STATUS_CODE = 1; }
                    }
                }
            }
        }
    }
    if unsafe { STATUS_CODE } != 0 {
        return;
    }

    // Use Tarjan's algorithm to detect a cycle.
    if detect_cycle(sheet, (r, c), total_rows, total_cols) {
        unsafe { STATUS_CODE = 3; }
        // Restore backup (swap dependents from backup).
        std::mem::swap(&mut backup.dependents, &mut sheet[r][c].dependents);
        sheet[r][c] = backup;
        return;
    }

    // Remove old dependencies from the backup cell.
    {
        match &backup.formula {
            Some(FormulaType::Range) => {
                if let (Some(old_r1), Some(old_r2)) = (backup.cell1.as_ref(), backup.cell2.as_ref()) {
                    let (start_row, start_col) = to_indices(old_r1);
                    let (end_row, end_col) = to_indices(old_r2);
                    for i in start_row..=end_row {
                        for j in start_col..=end_col {
                            remove_reference(&mut sheet[i][j], (r, c));
                        }
                    }
                }
            }
            _ => {
                if let Some(old_r1) = backup.cell1.as_ref() {
                    let (i, j) = to_indices(old_r1);
                    remove_reference(&mut sheet[i][j], (r, c));
                }
                if let Some(old_r2) = backup.cell2.as_ref() {
                    let (i, j) = to_indices(old_r2);
                    remove_reference(&mut sheet[i][j], (r, c));
                }
            }
        }
    }

    // Add new dependencies.
    {
        match &sheet[r][c].formula {
            Some(FormulaType::Range) => {
                let new_r1 = sheet[r][c].cell1.clone();
                let new_r2 = sheet[r][c].cell2.clone();
                if let (Some(ref start), Some(ref end)) = (new_r1, new_r2) {
                    add_range_dependencies(sheet, start, end, r, c);
                }
            }
            _ => {
                if let Some(new_r1) = &sheet[r][c].cell1 {
                    let (dep_row, dep_col) = to_indices(new_r1);
                    sheet[dep_row][dep_col].dependents.insert((r, c));
                }
                if let Some(new_r2) = &sheet[r][c].cell2 {
                    let (dep_row, dep_col) = to_indices(new_r2);
                    sheet[dep_row][dep_col].dependents.insert((r, c));
                }
            }
        }
    }
}
