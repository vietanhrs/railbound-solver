use std::collections::{HashMap, HashSet};
use crate::puzzle::{CellKind, Pos, Puzzle};
use crate::types::{Direction, TrackType};

const MAX_STEPS: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimResult {
    Win,
    /// A car fell off the grid or hit impassable track.
    Crash { car_color: u8, car_id: u8 },
    /// A car reached the caboose before the group was fully coupled.
    WrongOrder { car_color: u8, car_id: u8 },
    Loop,
}

#[derive(Clone, Debug)]
struct CarState {
    id: u8,
    color: u8,
    pos: Pos,
    dir: Direction,
    /// Index of the car in front of this one in the coupled chain (None = front/uncoupled).
    leader: Option<usize>,
    done: bool,
}

pub fn simulate(puzzle: &Puzzle, placements: &HashMap<Pos, TrackType>) -> SimResult {
    let mut cars: Vec<CarState> = puzzle.cars.iter().map(|c| CarState {
        id: c.id,
        color: c.color,
        pos: (c.row, c.col),
        dir: c.dir,
        leader: None,
        done: false,
    }).collect();

    // group_size[color] = total number of cars in that color group
    let mut group_size: HashMap<u8, u8> = HashMap::new();
    for c in &cars { *group_size.entry(c.color).or_insert(0) += 1; }

    let mut switches: HashMap<Pos, bool> = HashMap::new();
    // Loop detection: (pos, dir) set per car
    let mut visited: Vec<HashSet<(Pos, Direction)>> = vec![HashSet::new(); cars.len()];

    for _step in 0..MAX_STEPS {
        if cars.iter().all(|c| c.done) {
            return SimResult::Win;
        }

        // Save positions before this step (needed for coupling detection & coupled movement)
        let prev_pos: Vec<Pos> = cars.iter().map(|c| c.pos).collect();
        let prev_dir: Vec<Direction> = cars.iter().map(|c| c.dir).collect();

        // ── Move each active car ────────────────────────────────────────────
        for i in 0..cars.len() {
            if cars[i].done { continue; }

            if let Some(leader_idx) = cars[i].leader {
                // Coupled car: takes the position and direction its leader had at
                // the start of this step (i.e., prev_pos of leader).
                cars[i].pos = prev_pos[leader_idx];
                cars[i].dir = prev_dir[leader_idx];
            } else {
                // Independent car: follow track
                let pos = cars[i].pos;
                let dir = cars[i].dir;
                match advance_one(puzzle, placements, &mut switches, pos, dir) {
                    Some((new_pos, new_dir)) => {
                        let key = (new_pos, new_dir);
                        if !visited[i].insert(key) {
                            return SimResult::Loop;
                        }
                        cars[i].pos = new_pos;
                        cars[i].dir = new_dir;
                    }
                    None => return SimResult::Crash { car_color: cars[i].color, car_id: cars[i].id },
                }
            }
        }

        // ── Coupling detection ──────────────────────────────────────────────
        // Car (id = k+1) couples behind car (id = k) when:
        //   cars[j].pos == prev_pos[i]  (j arrived where i just was)
        //   cars[j].dir == cars[i].dir  (going the same direction)
        //   both same color, and j.id == i.id + 1
        //   and j is not already coupled, and i is a front car (no leader / or has lower id)
        for j in 0..cars.len() {
            if cars[j].done || cars[j].leader.is_some() { continue; }
            // Look for the car that should be directly in front of car j
            let front_id = cars[j].id.wrapping_sub(1); // id of the car in front
            if front_id == 0 { continue; } // id=1 is the front, nothing in front
            let color = cars[j].color;
            if let Some(i) = cars.iter().position(|c| {
                !c.done && c.color == color && c.id == front_id
            }) {
                if cars[j].pos == prev_pos[i] && cars[j].dir == cars[i].dir {
                    cars[j].leader = Some(i);
                }
            }
        }

        // ── Caboose detection ───────────────────────────────────────────────
        // The FRONT car (id=1) of each color group enters the caboose.
        // All other cars must be coupled (transitively behind car id=1) first.
        for i in 0..cars.len() {
            if cars[i].done { continue; }
            let pos = cars[i].pos;
            let color = cars[i].color;
            if let Some(cab) = puzzle.cabooses.iter().find(|c| c.color == color && c.row == pos.0 && c.col == pos.1) {
                // Only the front car (id=1) may enter the caboose
                if cars[i].id != 1 {
                    return SimResult::WrongOrder { car_color: color, car_id: cars[i].id };
                }
                // Must be approaching from the correct direction
                let required_travel_dir = cab.entry_side.opposite();
                if cars[i].dir != required_travel_dir {
                    return SimResult::Crash { car_color: color, car_id: cars[i].id };
                }
                // All cars in this group must be coupled behind this one
                let coupled_count = count_chain(&cars, i);
                let expected = group_size[&color] as usize;
                if coupled_count < expected {
                    return SimResult::WrongOrder { car_color: color, car_id: cars[i].id };
                }
                // Mark whole group done
                for car in cars.iter_mut() {
                    if car.color == color { car.done = true; }
                }
            }
        }
    }

    SimResult::Loop
}

/// Move a single car one cell forward from (pos, dir).
/// Returns new (pos, dir) or None on crash.
fn advance_one(
    puzzle: &Puzzle,
    placements: &HashMap<Pos, TrackType>,
    switches: &mut HashMap<Pos, bool>,
    pos: Pos,
    dir: Direction,
) -> Option<(Pos, Direction)> {
    let (dr, dc) = dir.delta();
    let nr = pos.0 as i32 + dr;
    let nc = pos.1 as i32 + dc;
    if !puzzle.in_bounds(nr, nc) { return None; }
    let np = (nr as usize, nc as usize);

    match &puzzle.grid[np.0][np.1] {
        CellKind::Tunnel(_) => {
            // Tunnel teleport: travel direction = dir when entering
            puzzle.tunnel_exit(np, dir).map(|(ep, ef)| (ep, ef))
        }
        CellKind::Terminal => {
            // Caboose / start cell: enter it (direction unchanged, no track needed)
            Some((np, dir))
        }
        CellKind::Wall => None,
        CellKind::Placeable | CellKind::Fixed(_) => {
            let track = puzzle.track_at(np, placements)?;
            let entry_side = dir.opposite();
            let flipped = *switches.get(&np).unwrap_or(&false);
            let (exit_side, toggle) = track.route(entry_side, flipped)?;
            if toggle {
                let s = switches.entry(np).or_insert(false);
                *s = !*s;
            }
            Some((np, exit_side))
        }
    }
}

/// Count how many cars are in the chain rooted at `leader_idx` (including leader).
fn count_chain(cars: &[CarState], leader_idx: usize) -> usize {
    let color = cars[leader_idx].color;
    let mut count = 1;
    for (j, car) in cars.iter().enumerate() {
        if j == leader_idx || car.done || car.color != color { continue; }
        let mut cur = j;
        let mut steps = 0;
        loop {
            if cur == leader_idx { count += 1; break; }
            match cars[cur].leader {
                Some(next) => { cur = next; steps += 1; if steps > cars.len() { break; } }
                None => break,
            }
        }
    }
    count
}

// ── Partial simulation for solver pruning ────────────────────────────────────

/// Quick forward-check: run cars through placed tracks.
/// Returns false if any car hits a definitive dead-end on placed/fixed track.
pub fn partial_simulate(puzzle: &Puzzle, placements: &HashMap<Pos, TrackType>) -> bool {
    let mut switches: HashMap<Pos, bool> = HashMap::new();
    for car in &puzzle.cars {
        let mut pos = (car.row, car.col);
        let mut dir = car.dir;
        let mut seen: HashSet<(Pos, Direction)> = HashSet::new();
        for _ in 0..MAX_STEPS {
            if !seen.insert((pos, dir)) { break; }
            let (dr, dc) = dir.delta();
            let nr = pos.0 as i32 + dr;
            let nc = pos.1 as i32 + dc;
            if !puzzle.in_bounds(nr, nc) { return false; }
            let np = (nr as usize, nc as usize);
            match &puzzle.grid[np.0][np.1] {
                CellKind::Wall => return false,
                CellKind::Terminal => break, // reached a caboose / start
                CellKind::Tunnel(_) => {
                    if let Some((ep, ef)) = puzzle.tunnel_exit(np, dir) {
                        pos = ep; dir = ef;
                    } else {
                        return false;
                    }
                    continue;
                }
                CellKind::Placeable if !placements.contains_key(&np) => break, // unresolved
                _ => {
                    if let Some(track) = puzzle.track_at(np, placements) {
                        let entry_side = dir.opposite();
                        let flipped = *switches.get(&np).unwrap_or(&false);
                        match track.route(entry_side, flipped) {
                            Some((exit_side, toggle)) => {
                                if toggle { let s = switches.entry(np).or_insert(false); *s = !*s; }
                                pos = np; dir = exit_side;
                            }
                            None => return false,
                        }
                    } else {
                        return false;
                    }
                }
            }
        }
    }
    true
}
