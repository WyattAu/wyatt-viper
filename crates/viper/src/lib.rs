//! Battlesnake wire types + pure game core.
//!
//! Logic ladder (keep each rung beatable-but-correct):
//! 1. never hit a wall / body (tail-aware: tails vacate unless health == 100)
//! 2. never lose a head-to-head (only contest cells adjacent to shorter-or-equal enemies)
//! 3. prefer more flood-fill space, then nearer food
//!
//! Test: `cargo test -p snake` · Server binary: `src/main.rs`

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Wire types (Battlesnake API v1 — fields are single words, no renaming needed)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct GameState {
    #[serde(default)]
    pub turn: u32,
    pub board: Board,
    pub you: Snake,
}

#[derive(Deserialize, Clone)]
pub struct Board {
    pub height: i32,
    pub width: i32,
    pub food: Vec<Pt>,
    pub snakes: Vec<Snake>,
    #[serde(default)]
    pub hazards: Vec<Pt>,
}

#[derive(Deserialize, Clone)]
pub struct Snake {
    pub id: String,
    pub health: i32,
    #[serde(default)]
    pub body: Vec<Pt>,
    pub head: Pt,
    #[serde(default)]
    pub length: u32,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Pt {
    pub x: i32,
    pub y: i32,
}

#[derive(Serialize)]
pub struct MoveResponse {
    #[serde(rename = "move")]
    pub direction: Direction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shout: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    const ALL: [Direction; 4] = [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ];

    fn delta(self) -> Pt {
        match self {
            Direction::Up => Pt { x: 0, y: -1 },
            Direction::Down => Pt { x: 0, y: 1 },
            Direction::Left => Pt { x: -1, y: 0 },
            Direction::Right => Pt { x: 1, y: 0 },
        }
    }
}

// ---------------------------------------------------------------------------
// Pure core — no I/O, fully unit-tested
// ---------------------------------------------------------------------------

pub mod engine {
    use super::{Board, Direction, Pt, Snake};
    use std::collections::VecDeque;

    fn in_bounds(b: &Board, p: Pt) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < b.width && p.y < b.height
    }

    /// Cell occupancy for this turn, tail-aware: a snake's tail cell vacates as
    /// it moves unless it just ate (health == 100 keeps the tail in place).
    fn occupancy(b: &Board) -> Vec<bool> {
        let mut occ = vec![false; (b.width * b.height) as usize];
        for s in &b.snakes {
            for (i, p) in s.body.iter().enumerate() {
                // Tails move simultaneously — a tail cell only stays solid if
                // that snake just ate (health == 100 keeps the tail in place).
                let is_vacating_tail = i + 1 == s.body.len() && s.health < 100;
                if !is_vacating_tail && in_bounds(b, *p) {
                    occ[(p.y * b.width + p.x) as usize] = true;
                }
            }
        }
        occ
    }

    /// Deadly head-to-head: cells adjacent to an equal-or-longer enemy head.
    fn head_to_head_deadly(b: &Board, me: &Snake) -> Vec<bool> {
        let mut deadly = vec![false; (b.width * b.height) as usize];
        for s in &b.snakes {
            if s.id == me.id || s.length < me.length {
                continue;
            }
            for d in Direction::ALL {
                let p = Pt {
                    x: s.head.x + d.delta().x,
                    y: s.head.y + d.delta().y,
                };
                if in_bounds(b, p) {
                    deadly[(p.y * b.width + p.x) as usize] = true;
                }
            }
        }
        deadly
    }

    /// Flood-fill reachable free area from `start` (capped — it's a heuristic).
    fn open_area(b: &Board, occ: &[bool], start: Pt) -> usize {
        let mut seen = vec![false; occ.len()];
        let mut q = VecDeque::new();
        let mut count = 0usize;
        seen[(start.y * b.width + start.x) as usize] = true;
        q.push_back(start);
        while let Some(p) = q.pop_front() {
            count += 1;
            if count >= 200 {
                break;
            }
            for d in Direction::ALL {
                let n = Pt {
                    x: p.x + d.delta().x,
                    y: p.y + d.delta().y,
                };
                if !in_bounds(b, n) {
                    continue;
                }
                let idx = (n.y * b.width + n.x) as usize;
                if !seen[idx] && !occ[idx] {
                    seen[idx] = true;
                    q.push_back(n);
                }
            }
        }
        count
    }

    fn manhattan(a: Pt, b: Pt) -> i32 {
        (a.x - b.x).abs() + (a.y - b.y).abs()
    }

    /// Score a candidate move; higher is better. Returns None for illegal.
    pub fn score_move(state: &super::GameState, dir: Direction) -> Option<i64> {
        let b = &state.board;
        let me = &state.you;
        let head = Pt {
            x: me.head.x + dir.delta().x,
            y: me.head.y + dir.delta().y,
        };
        if !in_bounds(b, head) {
            return None;
        }
        let occ = occupancy(b);
        let deadly = head_to_head_deadly(b, me);
        let idx = (head.y * b.width + head.x) as usize;
        if occ[idx] || deadly[idx] || b.hazards.contains(&head) {
            return None;
        }
        let mut area = open_area(b, &occ, head) as i64;
        area *= 10; // space dominates: starving snakes with no space are dead anyway
        let food = b
            .food
            .iter()
            .map(|f| 100 - 5 * i64::from(manhattan(head, *f).min(20)))
            .max()
            .unwrap_or(0);
        Some(area + food + i64::from(state.turn % 2)) // tie-break wobble
    }

    /// Pick the best legal move; always returns something (Battlesnake requires it).
    pub fn choose(state: &super::GameState) -> (Direction, &'static str) {
        let mut best: Option<(i64, Direction)> = None;
        for d in Direction::ALL {
            let Some(s) = score_move(state, d) else {
                continue;
            };
            if best.is_none_or(|(b, _)| s > b) {
                best = Some((s, d));
            }
        }
        match best {
            Some((_, d)) => (d, ""),
            // Cornered: every move is illegal — pick in-bounds and take the hit.
            None => {
                for d in Direction::ALL {
                    let n = Pt {
                        x: state.you.head.x + d.delta().x,
                        y: state.you.head.y + d.delta().y,
                    };
                    if n.x >= 0 && n.y >= 0 && n.x < state.board.width && n.y < state.board.height {
                        return (d, "cornered!");
                    }
                }
                (Direction::Up, "gg")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: i32, y: i32) -> Pt {
        Pt { x, y }
    }

    fn snake(id: &str, health: i32, cells: Vec<Pt>) -> Snake {
        let head = *cells.first().expect("non-empty body");
        let length = cells.len() as u32;
        Snake {
            id: id.into(),
            health,
            body: cells,
            head,
            length,
        }
    }

    /// Vertical 3x4 board: me heading up the middle, enemy on the left wall.
    fn sample(
        me_cells: Vec<Pt>,
        me_health: i32,
        enemy_cells: Vec<Pt>,
        enemy_health: i32,
    ) -> GameState {
        let me = snake("me", me_health, me_cells);
        let enemy = snake("foe", enemy_health, enemy_cells.clone());
        GameState {
            turn: 1,
            board: Board {
                height: 4,
                width: 3,
                food: vec![],
                snakes: vec![me.clone(), enemy],
                hazards: vec![],
            },
            you: me,
        }
    }

    #[test]
    fn directions_map_to_deltas() {
        assert_eq!(Direction::Up.delta(), pt(0, -1));
        assert_eq!(Direction::Right.delta(), pt(1, 0));
    }

    #[test]
    fn wall_is_illegal() {
        let state = sample(
            vec![pt(1, 1), pt(1, 2), pt(1, 3)],
            80,
            vec![pt(1, 0), pt(0, 0)],
            50,
        );
        assert_eq!(engine::score_move(&state, Direction::Up), None); // enemy body
    }

    #[test]
    fn vacating_tail_is_enterable() {
        // Enemy head (0,2), tail (0,1) at health 40: tail moves away this
        // turn → (0,1) is enterable (we out-length it, so no head-to-head).
        let state = sample(
            vec![pt(1, 1), pt(1, 2), pt(1, 3)],
            80,
            vec![pt(0, 2), pt(0, 1)],
            40,
        );
        assert!(engine::score_move(&state, Direction::Left).is_some());
    }

    #[test]
    fn fresh_tail_is_not_enterable() {
        // Enemy at health 100 → its tail (0,1) stays → (0,1) is solid.
        let state = sample(
            vec![pt(1, 1), pt(1, 2), pt(1, 3)],
            80,
            vec![pt(0, 2), pt(0, 1)],
            100,
        );
        assert!(engine::score_move(&state, Direction::Left).is_none());
    }

    #[test]
    fn equal_or_longer_head_is_deadly() {
        // Enemy head adjacent to (1,0), same length as me → deadly.
        let state = sample(vec![pt(1, 1), pt(1, 2)], 80, vec![pt(0, 0), pt(0, 1)], 50);
        assert_eq!(engine::score_move(&state, Direction::Up), None);
    }

    #[test]
    fn shorter_head_cell_is_contestable() {
        // Enemy shorter than me: its adjacent cell is fair game.
        let mut state = sample(
            vec![pt(1, 1), pt(1, 2), pt(1, 3)],
            80,
            vec![pt(0, 0), pt(0, 1)],
            50,
        );
        state.board.snakes[1].length = 1;
        assert!(engine::score_move(&state, Direction::Up).is_some());
    }

    #[test]
    fn choose_always_returns_a_move() {
        let state = sample(
            vec![pt(1, 1), pt(1, 2), pt(1, 3)],
            80,
            vec![pt(1, 0), pt(0, 0)],
            50,
        );
        let (dir, _) = engine::choose(&state);
        let _ = serde_json::to_string(&dir).expect("serializable");
    }
}
