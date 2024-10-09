use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Copy, Clone, Debug)]
pub enum LedState {
    Init = 0,
    Party = 1,
    Clear = 2,
    Error = 3,
    Warning = 4,
}

impl LedState {
    fn from_usize(value: usize) -> Self {
        match value {
            0 => LedState::Init,
            1 => LedState::Party,
            2 => LedState::Clear,
            3 => LedState::Error,
            4 => LedState::Warning,
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
        let next_state = self.load(ordering) as usize + 1;
        if next_state > LedState::Warning as usize {
            self.store(LedState::Init, ordering);
        } else {
            self.store(LedState::from_usize(next_state), ordering);
        }
    }
}
