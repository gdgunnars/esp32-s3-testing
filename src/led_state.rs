use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Copy, Clone, Debug)]
pub enum LedState {
    INIT = 0,
    PARTY = 1,
    CLEAR = 2,
    ERROR = 3,
    WARNING = 4,
}

impl LedState {
    fn from_usize(value: usize) -> Self {
        match value {
            0 => LedState::INIT,
            1 => LedState::PARTY,
            2 => LedState::CLEAR,
            3 => LedState::ERROR,
            4 => LedState::WARNING,
            _ => panic!("Invalid value for LedState"),
        }
    }
}

// Define AtomicLedState to wrap AtomicUsize
pub struct AtomicLedState {
    state: AtomicUsize,
}

impl AtomicLedState {
    pub fn new(initial_state: LedState) -> Self {
        AtomicLedState {
            state: AtomicUsize::new(initial_state as usize),
        }
    }

    // Load the current state atomically, returning the LedState enum
    pub fn load(&self, ordering: Ordering) -> LedState {
        let state = self.state.load(ordering);
        LedState::from_usize(state)
    }

    // Store a new state atomically
    pub fn store(&self, state: LedState, ordering: Ordering) {
        self.state.store(state as usize, ordering);
    }

    // Increment the state, wrapping if necessary
    // Used for testing the atomic state
    pub fn increment(&self, ordering: Ordering) {
        let current_state = self.load(ordering) as usize + 1;
        if current_state > LedState::WARNING as usize {
            self.store(LedState::PARTY, ordering);
        } else {
            self.store(LedState::from_usize(current_state), ordering);
        }
    }
}
