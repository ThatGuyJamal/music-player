use std::sync::{Arc, RwLock};

use dashmap::DashMap;

use crate::player::Player;

type PlayerId = String;

/// Manages all the current players in the app.
pub struct Music {
    pub players: Arc<DashMap<PlayerId, Arc<RwLock<Player>>>>,
}

impl Music {
    pub fn new() -> Self {
        Self {
            players: Arc::new(DashMap::new()),
        }
    }

    /// Adds a player to the app memory
    pub fn add_player(&self, key: String, player: Player) {
        self.players.insert(key, Arc::new(RwLock::new(player)));
    }

    pub fn remove_player(&self, key: &str) {
        self.players.remove(key);
    }

    pub fn get_player(&self, key: &str) -> Option<Arc<RwLock<Player>>> {
        self.players.get(key).map(|entry| Arc::clone(entry.value()))
    }

    pub fn list_players(&self) -> Vec<(String, Arc<RwLock<Player>>)> {
        self.players
            .iter()
            .map(|entry| (entry.key().clone(), Arc::clone(entry.value())))
            .collect()
    }
}
