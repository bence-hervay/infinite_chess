use crate::arena::enumerate;
use crate::game::Game;
use crate::pieces::Turn;
use crate::rules::movegen::{self, Scratch, Succ};
use crate::state::PackedState;
use std::collections::HashMap;

pub type NodeId = usize;

#[derive(Clone, Debug)]
pub struct Node {
    pub turn: Turn,
    /// None indicates a sink node.
    pub state: Option<PackedState>,
    pub succ: Vec<NodeId>,
    pub pred: Vec<NodeId>,
}

#[derive(Clone, Debug)]
pub struct Arena {
    pub game: Game,
    pub nodes: Vec<Node>,
    pub sink_black: NodeId,
    pub sink_white: NodeId,
}

impl Arena {
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_sink(&self, id: NodeId) -> bool {
        id == self.sink_black || id == self.sink_white
    }
}

pub struct ArenaBuilder {
    pub game: Game,
}

impl ArenaBuilder {
    pub fn new(game: Game) -> Self {
        Self { game }
    }

    /// Enumerate all legal states in the region and build the full turn-based game graph.
    pub fn enumerate_all(&self) -> Arena {
        let game = self.game.clone();
        let states = enumerate::all_states(&game);

        let mut state_to_index: HashMap<PackedState, usize> = HashMap::with_capacity(states.len());
        for (i, st) in states.iter().enumerate() {
            state_to_index.insert(*st, i);
        }

        let sink_black: NodeId = 0;
        let sink_white: NodeId = 1;

        let mut nodes: Vec<Node> = Vec::with_capacity(2 + 2 * states.len());
        nodes.push(Node {
            turn: Turn::Black,
            state: None,
            succ: vec![sink_black],
            pred: Vec::new(),
        });
        nodes.push(Node {
            turn: Turn::White,
            state: None,
            succ: vec![sink_white],
            pred: Vec::new(),
        });

        // Add nodes for each state in a fixed order: black node then white node.
        for st in &states {
            nodes.push(Node {
                turn: Turn::Black,
                state: Some(*st),
                succ: Vec::new(),
                pred: Vec::new(),
            });
            nodes.push(Node {
                turn: Turn::White,
                state: Some(*st),
                succ: Vec::new(),
                pred: Vec::new(),
            });
        }

        let mut scratch = Scratch::new(game.layout.total_white());

        // Fill edges.
        for id in 2..nodes.len() {
            let turn = nodes[id].turn;
            let st = nodes[id].state.expect("non-sink nodes have a state");

            let succs = movegen::successors(&game, turn, st, &mut scratch);
            let mut succ_ids: Vec<NodeId> = Vec::with_capacity(succs.len());

            for s in succs {
                match s {
                    Succ::Sink => {
                        let sink = match turn {
                            Turn::Black => sink_white,
                            Turn::White => sink_black,
                        };
                        succ_ids.push(sink);
                    }
                    Succ::State(next_state) => {
                        let next_turn = turn.other();
                        let idx = *state_to_index
                            .get(&next_state)
                            .unwrap_or_else(|| panic!("successor state missing from enumeration: {next_state}"));
                        let base = 2 + 2 * idx;
                        let next_id = match next_turn {
                            Turn::Black => base,
                            Turn::White => base + 1,
                        };
                        succ_ids.push(next_id);
                    }
                }
            }

            // Determinise the successor list.
            succ_ids.sort_unstable();
            succ_ids.dedup();

            nodes[id].succ = succ_ids.clone();

            for s in succ_ids {
                nodes[s].pred.push(id);
            }
        }

        Arena {
            game,
            nodes,
            sink_black,
            sink_white,
        }
    }
}
