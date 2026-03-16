# railbound-solver

An automatic solver for the [Railbound](https://store.steampowered.com/app/1967510/Railbound/) puzzle game, written in Rust.

You describe a puzzle in a plain-text `.rail` file and the solver finds a valid track placement, displaying the solution in your terminal using Unicode box-drawing characters.

## Features

- All core Railbound mechanics:
  - Straight tracks, 4 curve types, crossings
  - Y-switches (8 orientations) with toggle state
  - Tunnels (teleportation between two entrances)
  - Multi-car coupling (cars must couple in numbered order before reaching the caboose)
  - Multi-color train groups (each color solved simultaneously)
- DFS backtracking solver with pruning:
  - Cells ordered by BFS distance from train/caboose positions (most-constrained first)
  - Partial simulation: prune if any car hits a dead-end on placed track
  - Connectivity check: prune if a car can no longer reach its caboose
- Unicode visual output

## Usage

```
cargo run --release -- <puzzle.rail>
```

Example output:

```
Puzzle  (6 cols × 3 rows)
┌─────────────────────────┐
│    │   │   │   │   │    │
├────┼───┼───┼───┼───┼────┤
│ →1 │ ░ │ ░ │ ░ │ ░ │ C1 │
├────┼───┼───┼───┼───┼────┤
│ →2 │ ░ │ ░ │ ░ │ ░ │    │
└─────────────────────────┘

Solution found!
┌─────────────────────────┐
│    │   │   │   │   │    │
├────┼───┼───┼───┼───┼────┤
│ →1 │ ═ │ ╩ │ ═ │ ═ │ C1 │
├────┼───┼───┼───┼───┼────┤
│ →2 │ ═ │ ╝ │ ░ │ ░ │    │
└─────────────────────────┘
```

## Puzzle file format

```
# Comments start with #
SIZE <cols> <rows>

GRID
<cell> <cell> ...    # one row per line, cells separated by spaces

TRAINS
# id  row  col  dir  color
1     1    0    E    1

CABOOSES
# color  row  col  entry_dir
1        1    5    W

TUNNELS
# label  entry_row  entry_col  entry_facing  exit_row  exit_col  exit_facing
a        0          1          E             2         1         E

PIECES
straight  <n>
curve     <n>
crossing  <n>
switch    <n>
```

### Grid cell codes

| Code | Meaning |
|------|---------|
| `.`  | Wall (impassable) |
| `_`  | Empty — solver may place a piece here |
| `H`  | Fixed horizontal straight (═) |
| `V`  | Fixed vertical straight (║) |
| `NE` | Fixed curve ╚ (North & East open) |
| `NW` | Fixed curve ╝ (North & West open) |
| `SE` | Fixed curve ╔ (South & East open) |
| `SW` | Fixed curve ╗ (South & West open) |
| `X`  | Fixed crossing (╬) |
| `Y<t><b>` | Fixed switch — trunk=`t`, branch=`b`, e.g. `YWN` |
| `T<l>` | Tunnel entrance/exit with label `l` (e.g. `Ta`) |

Directions: `N` `S` `E` `W`

### Coupling rules

- Car **id=1** is the front car and must enter the caboose last (after all others have coupled behind it).
- Car **id=k+1** couples behind car **id=k** when it steps onto the cell that car k just vacated, traveling in the same direction.
- A switch is the typical mechanism for merging two cars onto the same track.

## Examples

Three example puzzles are in the `puzzles/` directory:

| File | Description |
|------|-------------|
| `puzzles/example0.rail` | Single car, straight track |
| `puzzles/example1.rail` | Two cars merging via a switch |
| `puzzles/example2.rail` | Single car with tunnel teleportation |
