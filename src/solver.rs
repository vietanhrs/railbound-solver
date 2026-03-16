use std::collections::HashMap;
use crate::puzzle::{CellKind, Pos, Puzzle};
use crate::simulation::{partial_simulate, simulate, SimResult};
use crate::types::{Direction, Inventory, TrackType};

/// The solved placement: which track to put at each previously-empty cell.
pub type Solution = HashMap<Pos, TrackType>;

/// Entry point: find a track placement that solves the puzzle.
/// Returns `None` if no solution exists.
pub fn solve(puzzle: &Puzzle) -> Option<Solution> {
    // Order placeable cells: BFS from train positions (most-constrained first)
    let cells = ordered_cells(puzzle);
    let mut placements: Solution = HashMap::new();
    let mut inv = puzzle.inventory.clone();

    if dfs(puzzle, &cells, 0, &mut inv, &mut placements) {
        Some(placements)
    } else {
        None
    }
}

/// DFS over the ordered list of placeable cells.
/// At each cell we try placing each available piece type/orientation, or leaving it empty.
fn dfs(
    puzzle: &Puzzle,
    cells: &[Pos],
    idx: usize,
    inv: &mut Inventory,
    placements: &mut Solution,
) -> bool {
    // Base case: all cells considered → run full simulation
    if idx == cells.len() {
        return simulate(puzzle, placements) == SimResult::Win;
    }

    let pos = cells[idx];

    // Collect candidate track types (all orientations for each available piece type)
    // plus the "leave empty" option.
    let candidates = build_candidates(inv);

    for candidate in candidates {
        match candidate {
            None => {
                // Leave this cell empty — just recurse
                if dfs(puzzle, cells, idx + 1, inv, placements) {
                    return true;
                }
            }
            Some((track, piece_kind)) => {
                // Place this track, decrement inventory
                placements.insert(pos, track);
                decrement(inv, piece_kind);

                // Pruning: check if any car is already on a definitive crash path
                if partial_simulate(puzzle, placements) {
                    // Connectivity pruning: each car should still be able to reach
                    // its caboose through the current + remaining unplaced cells
                    if connectivity_ok(puzzle, placements) {
                        if dfs(puzzle, cells, idx + 1, inv, placements) {
                            return true;
                        }
                    }
                }

                // Undo
                placements.remove(&pos);
                increment(inv, piece_kind);
            }
        }
    }

    false
}

/// Build the list of (TrackType, piece_kind) pairs to try, plus None for "leave empty".
/// We deduplicate: only list a piece kind once per availability.
fn build_candidates(inv: &Inventory) -> Vec<Option<(TrackType, PieceKind)>> {
    let mut v: Vec<Option<(TrackType, PieceKind)>> = Vec::new();

    // "Leave empty" first — often the right choice for most cells
    v.push(None);

    if inv.straight > 0 {
        for &t in TrackType::straight_orientations() {
            v.push(Some((t, PieceKind::Straight)));
        }
    }
    if inv.curve > 0 {
        for &t in TrackType::curve_orientations() {
            v.push(Some((t, PieceKind::Curve)));
        }
    }
    if inv.crossing > 0 {
        for &t in TrackType::crossing_orientations() {
            v.push(Some((t, PieceKind::Crossing)));
        }
    }
    if inv.switch > 0 {
        for t in TrackType::switch_orientations() {
            v.push(Some((t, PieceKind::Switch)));
        }
    }

    v
}

#[derive(Clone, Copy)]
enum PieceKind { Straight, Curve, Crossing, Switch }

fn decrement(inv: &mut Inventory, kind: PieceKind) {
    match kind {
        PieceKind::Straight => inv.straight -= 1,
        PieceKind::Curve    => inv.curve    -= 1,
        PieceKind::Crossing => inv.crossing -= 1,
        PieceKind::Switch   => inv.switch   -= 1,
    }
}

fn increment(inv: &mut Inventory, kind: PieceKind) {
    match kind {
        PieceKind::Straight => inv.straight += 1,
        PieceKind::Curve    => inv.curve    += 1,
        PieceKind::Crossing => inv.crossing += 1,
        PieceKind::Switch   => inv.switch   += 1,
    }
}

// ── Cell ordering: BFS from all train positions ───────────────────────────────

fn ordered_cells(puzzle: &Puzzle) -> Vec<Pos> {
    use std::collections::VecDeque;

    let mut dist: HashMap<Pos, usize> = HashMap::new();
    let mut queue: VecDeque<Pos> = VecDeque::new();

    for car in &puzzle.cars {
        let pos = (car.row, car.col);
        if dist.insert(pos, 0).is_none() {
            queue.push_back(pos);
        }
    }
    for cab in &puzzle.cabooses {
        let pos = (cab.row, cab.col);
        if dist.insert(pos, 0).is_none() {
            queue.push_back(pos);
        }
    }

    while let Some(pos) = queue.pop_front() {
        let d = dist[&pos];
        for dir in Direction::all() {
            let (dr, dc) = dir.delta();
            let nr = pos.0 as i32 + dr;
            let nc = pos.1 as i32 + dc;
            if !puzzle.in_bounds(nr, nc) {
                continue;
            }
            let np = (nr as usize, nc as usize);
            if !dist.contains_key(&np) {
                match &puzzle.grid[np.0][np.1] {
                    CellKind::Wall => {} // don't traverse
                    _ => {
                        dist.insert(np, d + 1);
                        queue.push_back(np);
                    }
                }
            }
        }
    }

    let mut placeable: Vec<Pos> = puzzle.placeable_cells();
    // Sort by BFS distance from trains/cabooses (closer = more constrained → try first)
    placeable.sort_by_key(|p| dist.get(p).copied().unwrap_or(usize::MAX));
    placeable
}

// ── Connectivity pruning ──────────────────────────────────────────────────────

/// Check that every car can still reach its caboose through the current placement
/// plus all remaining unplaced cells (treated as passable wildcards).
/// This is a necessary (not sufficient) condition for solvability.
fn connectivity_ok(puzzle: &Puzzle, placements: &Solution) -> bool {
    for car in &puzzle.cars {
        let caboose = puzzle
            .cabooses
            .iter()
            .find(|c| c.color == car.color);
        let Some(cab) = caboose else { continue };

        let target = (cab.row, cab.col);
        if !can_reach(puzzle, placements, (car.row, car.col), target) {
            return false;
        }
    }
    true
}

/// BFS reachability: can we get from `start` to `goal` treating
/// unplaced placeable cells as "connects any sides" (wildcard)?
fn can_reach(
    puzzle: &Puzzle,
    placements: &Solution,
    start: Pos,
    goal: Pos,
) -> bool {
    use std::collections::VecDeque;

    if start == goal {
        return true;
    }

    let mut visited: HashMap<Pos, ()> = HashMap::new();
    let mut queue: VecDeque<Pos> = VecDeque::new();

    visited.insert(start, ());
    queue.push_back(start);

    while let Some(pos) = queue.pop_front() {
        for dir in Direction::all() {
            let (dr, dc) = dir.delta();
            let nr = pos.0 as i32 + dr;
            let nc = pos.1 as i32 + dc;
            if !puzzle.in_bounds(nr, nc) {
                continue;
            }
            let np = (nr as usize, nc as usize);
            if visited.contains_key(&np) {
                continue;
            }

            let passable = match &puzzle.grid[np.0][np.1] {
                CellKind::Wall => false,
                CellKind::Placeable => !placements.contains_key(&np)
                    || track_connects_from(placements.get(&np).unwrap(), dir.opposite()),
                CellKind::Fixed(t) => track_connects_from(t, dir.opposite()),
                CellKind::Tunnel(_) => true, // tunnels are passable (teleport handled below)
                CellKind::Terminal => np == goal, // only goal terminal is useful
            };

            if passable {
                if np == goal {
                    return true;
                }
                // If entering a tunnel, also enqueue the teleport destination
                if let CellKind::Tunnel(_) = &puzzle.grid[np.0][np.1] {
                    // Try all entry directions for this tunnel
                    for &entry_dir in &[crate::types::Direction::North, crate::types::Direction::South,
                                        crate::types::Direction::East,  crate::types::Direction::West] {
                        if let Some((exit_pos, _)) = puzzle.tunnel_exit(np, entry_dir) {
                            if !visited.contains_key(&exit_pos) {
                                if exit_pos == goal { return true; }
                                visited.insert(exit_pos, ());
                                queue.push_back(exit_pos);
                            }
                        }
                    }
                }
                visited.insert(np, ());
                queue.push_back(np);
            }
        }
    }

    false
}

/// Check if a track piece has an opening on `entry_side` (i.e. a train can enter from that side).
fn track_connects_from(track: &TrackType, entry_side: Direction) -> bool {
    track.open_sides().contains(&entry_side)
}
