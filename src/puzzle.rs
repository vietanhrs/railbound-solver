use std::collections::HashMap;
use std::fmt;
use crate::types::{Direction, Inventory, TrackType};

pub type Pos = (usize, usize); // (row, col)

/// What is fixed (pre-placed) in a grid cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CellKind {
    /// Impassable — no track can be placed here.
    Wall,
    /// Empty cell where the solver may place a track piece.
    Placeable,
    /// A pre-placed, immovable track piece.
    Fixed(TrackType),
    /// A tunnel entrance/exit labelled by an ASCII letter.
    Tunnel(char),
    /// Occupied by a train car or caboose — treated as terminal; no track placed here.
    Terminal,
}

/// A single train car.
#[derive(Clone, Debug)]
pub struct Car {
    /// Unique id within its color group (1, 2, 3, …).
    pub id: u8,
    /// Color group identifier.
    pub color: u8,
    pub row: usize,
    pub col: usize,
    /// The direction the car is initially facing / moving.
    pub dir: Direction,
}

/// A caboose (terminal destination for a color group).
#[derive(Clone, Debug)]
pub struct Caboose {
    pub color: u8,
    pub row: usize,
    pub col: usize,
    /// The side of the caboose cell from which trains enter.
    /// A train moving East enters the caboose's West side, so entry_side = West.
    pub entry_side: Direction,
}

/// A tunnel pair: train enters `entry_pos` moving `entry_facing`, exits `exit_pos` moving `exit_facing`.
#[derive(Clone, Debug)]
pub struct Tunnel {
    pub label: char,
    pub entry_pos: Pos,
    pub entry_facing: Direction,
    pub exit_pos: Pos,
    pub exit_facing: Direction,
}

/// The complete puzzle description.
#[derive(Clone, Debug)]
pub struct Puzzle {
    pub rows: usize,
    pub cols: usize,
    /// `grid[row][col]` — fixed cell contents.
    pub grid: Vec<Vec<CellKind>>,
    pub cars: Vec<Car>,
    pub cabooses: Vec<Caboose>,
    pub tunnels: Vec<Tunnel>,
    pub inventory: Inventory,
}

impl Puzzle {
    /// Positions of all placeable cells, in row-major order.
    pub fn placeable_cells(&self) -> Vec<Pos> {
        let mut cells = Vec::new();
        for r in 0..self.rows {
            for c in 0..self.cols {
                if self.grid[r][c] == CellKind::Placeable {
                    cells.push((r, c));
                }
            }
        }
        cells
    }

    /// Lookup the effective track at a position, checking both fixed cells and solver placements.
    pub fn track_at<'a>(
        &'a self,
        pos: Pos,
        placements: &'a HashMap<Pos, TrackType>,
    ) -> Option<&'a TrackType> {
        if let Some(t) = placements.get(&pos) {
            return Some(t);
        }
        match &self.grid[pos.0][pos.1] {
            CellKind::Fixed(t) => Some(t),
            _ => None,
        }
    }

    /// Check if a position is in bounds.
    pub fn in_bounds(&self, row: i32, col: i32) -> bool {
        row >= 0 && col >= 0 && row < self.rows as i32 && col < self.cols as i32
    }

    /// Look up tunnel: given an entry position and facing direction, return the exit.
    pub fn tunnel_exit(&self, pos: Pos, facing: Direction) -> Option<(Pos, Direction)> {
        for t in &self.tunnels {
            if t.entry_pos == pos && t.entry_facing == facing {
                return Some((t.exit_pos, t.exit_facing));
            }
            // Tunnels are bidirectional: swap entry/exit
            if t.exit_pos == pos && t.exit_facing == facing {
                return Some((t.entry_pos, t.entry_facing));
            }
        }
        None
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parse error: {}", self.0)
    }
}

macro_rules! err {
    ($($t:tt)*) => { Err(ParseError(format!($($t)*))) }
}

/// Parse a puzzle from the text of a `.rail` file.
///
/// # File Format
///
/// ```text
/// # Comments start with #
/// SIZE <cols> <rows>
///
/// GRID
/// <cell> <cell> ...     # one row per line, cells separated by spaces
/// ...
///
/// TRAINS
/// # <id> <row> <col> <dir> <color>
/// 1 0 0 E 1
///
/// CABOOSES
/// # <color> <row> <col> <entry_dir>
/// 1 0 5 W
///
/// TUNNELS
/// # <label> <entry_row> <entry_col> <entry_facing> <exit_row> <exit_col> <exit_facing>
/// a 0 2 S 3 4 N
///
/// PIECES
/// straight <n>
/// curve    <n>
/// crossing <n>
/// switch   <n>
/// ```
///
/// ## Grid cell codes
/// - `.`  — wall (impassable)
/// - `_`  — empty, solver may place a piece here
/// - `H`  — fixed StraightH
/// - `V`  — fixed StraightV
/// - `NE` — fixed CurveNE (╚)
/// - `NW` — fixed CurveNW (╝)
/// - `SE` — fixed CurveSE (╔)
/// - `SW` — fixed CurveSW (╗)
/// - `X`  — fixed Crossing
/// - `Y<trunk><branch>` — fixed switch, e.g., `YWN` = trunk=W, branch=N
/// - `Ta`, `Tb`, … — tunnel (label defined in TUNNELS section)
pub fn parse_puzzle(input: &str) -> Result<Puzzle, ParseError> {
    let mut section: Option<&str> = None;
    let mut cols = 0usize;
    let mut rows = 0usize;
    let mut grid_rows: Vec<Vec<CellKind>> = Vec::new();
    let mut cars: Vec<Car> = Vec::new();
    let mut cabooses: Vec<Caboose> = Vec::new();
    let mut tunnels: Vec<Tunnel> = Vec::new();
    let mut inventory = Inventory::default();

    for (line_no, raw_line) in input.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }

        // Section headers
        match line {
            "GRID"     => { section = Some("GRID");     continue; }
            "TRAINS"   => { section = Some("TRAINS");   continue; }
            "CABOOSES" => { section = Some("CABOOSES"); continue; }
            "TUNNELS"  => { section = Some("TUNNELS");  continue; }
            "PIECES"   => { section = Some("PIECES");   continue; }
            _ => {}
        }

        // SIZE directive (can appear before sections)
        if line.starts_with("SIZE") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                return err!("line {}: SIZE requires two numbers", line_no + 1);
            }
            cols = parts[1].parse().map_err(|_| ParseError(format!("line {}: bad cols", line_no + 1)))?;
            rows = parts[2].parse().map_err(|_| ParseError(format!("line {}: bad rows", line_no + 1)))?;
            continue;
        }

        match section {
            Some("GRID") => {
                if cols == 0 {
                    return err!("line {}: SIZE must appear before GRID", line_no + 1);
                }
                let mut row_cells = Vec::new();
                for token in line.split_whitespace() {
                    let cell = parse_cell(token)
                        .ok_or_else(|| ParseError(format!("line {}: unknown cell code '{}'", line_no + 1, token)))?;
                    row_cells.push(cell);
                }
                if row_cells.len() != cols {
                    return err!(
                        "line {}: expected {} cells, got {}",
                        line_no + 1, cols, row_cells.len()
                    );
                }
                grid_rows.push(row_cells);
            }

            Some("TRAINS") => {
                // <id> <row> <col> <dir> <color>
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 5 {
                    return err!("line {}: TRAIN needs: id row col dir color", line_no + 1);
                }
                let id: u8  = parts[0].parse().map_err(|_| ParseError(format!("line {}: bad id", line_no + 1)))?;
                let row: usize = parts[1].parse().map_err(|_| ParseError(format!("line {}: bad row", line_no + 1)))?;
                let col: usize = parts[2].parse().map_err(|_| ParseError(format!("line {}: bad col", line_no + 1)))?;
                let dir = Direction::from_str(parts[3])
                    .ok_or_else(|| ParseError(format!("line {}: bad direction '{}'", line_no + 1, parts[3])))?;
                let color: u8 = parts[4].parse().map_err(|_| ParseError(format!("line {}: bad color", line_no + 1)))?;
                cars.push(Car { id, color, row, col, dir });
            }

            Some("CABOOSES") => {
                // <color> <row> <col> <entry_dir>
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 4 {
                    return err!("line {}: CABOOSE needs: color row col entry_dir", line_no + 1);
                }
                let color: u8 = parts[0].parse().map_err(|_| ParseError(format!("line {}: bad color", line_no + 1)))?;
                let row: usize = parts[1].parse().map_err(|_| ParseError(format!("line {}: bad row", line_no + 1)))?;
                let col: usize = parts[2].parse().map_err(|_| ParseError(format!("line {}: bad col", line_no + 1)))?;
                let entry_side = Direction::from_str(parts[3])
                    .ok_or_else(|| ParseError(format!("line {}: bad direction '{}'", line_no + 1, parts[3])))?;
                cabooses.push(Caboose { color, row, col, entry_side });
            }

            Some("TUNNELS") => {
                // <label> <er> <ec> <ef> <xr> <xc> <xf>
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 7 {
                    return err!("line {}: TUNNEL needs: label er ec ef xr xc xf", line_no + 1);
                }
                let label = parts[0].chars().next()
                    .ok_or_else(|| ParseError(format!("line {}: empty tunnel label", line_no + 1)))?;
                let er: usize = parts[1].parse().map_err(|_| ParseError(format!("line {}: bad row", line_no + 1)))?;
                let ec: usize = parts[2].parse().map_err(|_| ParseError(format!("line {}: bad col", line_no + 1)))?;
                let ef = Direction::from_str(parts[3])
                    .ok_or_else(|| ParseError(format!("line {}: bad direction", line_no + 1)))?;
                let xr: usize = parts[4].parse().map_err(|_| ParseError(format!("line {}: bad row", line_no + 1)))?;
                let xc: usize = parts[5].parse().map_err(|_| ParseError(format!("line {}: bad col", line_no + 1)))?;
                let xf = Direction::from_str(parts[6])
                    .ok_or_else(|| ParseError(format!("line {}: bad direction", line_no + 1)))?;
                tunnels.push(Tunnel {
                    label,
                    entry_pos: (er, ec),
                    entry_facing: ef,
                    exit_pos: (xr, xc),
                    exit_facing: xf,
                });
            }

            Some("PIECES") => {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 2 {
                    return err!("line {}: PIECES needs: type count", line_no + 1);
                }
                let count: usize = parts[1].parse()
                    .map_err(|_| ParseError(format!("line {}: bad count", line_no + 1)))?;
                match parts[0].to_lowercase().as_str() {
                    "straight" => inventory.straight += count,
                    "curve"    => inventory.curve    += count,
                    "crossing" => inventory.crossing += count,
                    "switch"   => inventory.switch   += count,
                    other => return err!("line {}: unknown piece type '{}'", line_no + 1, other),
                }
            }

            _ => {
                return err!("line {}: unexpected content outside of a section", line_no + 1);
            }
        }
    }

    if rows == 0 || cols == 0 {
        return err!("SIZE not specified");
    }
    if grid_rows.len() != rows {
        return err!("expected {} grid rows, got {}", rows, grid_rows.len());
    }

    // Mark cells occupied by cars/cabooses as Terminal so the solver won't place tracks there
    let mut grid = grid_rows;
    for car in &cars {
        if car.row < rows && car.col < cols {
            grid[car.row][car.col] = CellKind::Terminal;
        }
    }
    for cab in &cabooses {
        if cab.row < rows && cab.col < cols {
            grid[cab.row][cab.col] = CellKind::Terminal;
        }
    }

    Ok(Puzzle { rows, cols, grid, cars, cabooses, tunnels, inventory })
}

fn parse_cell(token: &str) -> Option<CellKind> {
    match token {
        "." => Some(CellKind::Wall),
        "_" => Some(CellKind::Placeable),
        other => {
            // Fixed track type?
            if let Some(tt) = TrackType::from_code(other) {
                return Some(CellKind::Fixed(tt));
            }
            // Tunnel label: T followed by a single letter, e.g. "Ta", "Tb"
            if other.starts_with('T') && other.len() == 2 {
                let label = other.chars().nth(1)?;
                return Some(CellKind::Tunnel(label));
            }
            None
        }
    }
}
