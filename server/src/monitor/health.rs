use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct HeartbeatData {
    pub world_id: String,
    pub last_heartbeat: Instant,
    pub current_players: u32,
}

impl HeartbeatData {
    pub fn new(world_id: String, current_players: u32) -> Self {
        Self {
            world_id,
            last_heartbeat: Instant::now(),
            current_players,
        }
    }

    pub fn is_online(&self) -> bool {
        Instant::now().duration_since(self.last_heartbeat) < HEARTBEAT_TIMEOUT
    }

    pub fn update(&mut self, current_players: u32) {
        self.last_heartbeat = Instant::now();
        self.current_players = current_players;
    }
}

#[derive(Clone)]
pub struct HealthMonitor {
    heartbeats: Arc<DashMap<String, HeartbeatData>>,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            heartbeats: Arc::new(DashMap::new()),
        }
    }

    pub fn record_heartbeat(&self, world_id: String, current_players: u32) {
        if let Some(mut heartbeat) = self.heartbeats.get_mut(&world_id) {
            heartbeat.update(current_players);
            log::debug!(
                "Updated heartbeat for world {} with {} players",
                world_id,
                current_players
            );
        } else {
            let heartbeat = HeartbeatData::new(world_id.clone(), current_players);
            self.heartbeats.insert(world_id.clone(), heartbeat);
            log::info!(
                "First heartbeat received for world {} with {} players",
                world_id,
                current_players
            );
        }
    }

    pub fn is_world_online(&self, world_id: &str) -> bool {
        self.heartbeats
            .get(world_id)
            .map(|h| h.is_online())
            .unwrap_or(false)
    }

    pub fn get_world_status(&self, world_id: &str) -> Option<(bool, u32)> {
        self.heartbeats
            .get(world_id)
            .map(|h| (h.is_online(), h.current_players))
    }

    pub fn cleanup_stale_heartbeats(&self) -> usize {
        let mut removed = 0;

        self.heartbeats.retain(|world_id, heartbeat| {
            if !heartbeat.is_online() {
                log::info!("World {} marked as offline (no heartbeat)", world_id);
                removed += 1;
                false
            } else {
                true
            }
        });

        if removed > 0 {
            log::info!("Cleaned up {} stale heartbeats", removed);
        }

        removed
    }

    pub fn get_all_online_worlds(&self) -> Vec<(String, u32)> {
        self.heartbeats
            .iter()
            .filter(|entry| entry.value().is_online())
            .map(|entry| (entry.key().clone(), entry.value().current_players))
            .collect()
    }

    pub fn online_world_count(&self) -> usize {
        self.heartbeats
            .iter()
            .filter(|entry| entry.value().is_online())
            .count()
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_heartbeat() {
        let monitor = HealthMonitor::new();
        monitor.record_heartbeat("world-1".to_string(), 42);

        assert!(monitor.is_world_online("world-1"));
        assert_eq!(monitor.get_world_status("world-1"), Some((true, 42)));
    }

    #[test]
    fn test_update_heartbeat() {
        let monitor = HealthMonitor::new();
        monitor.record_heartbeat("world-1".to_string(), 42);
        monitor.record_heartbeat("world-1".to_string(), 50);

        assert_eq!(monitor.get_world_status("world-1"), Some((true, 50)));
    }

    #[test]
    fn test_world_offline_no_heartbeat() {
        let monitor = HealthMonitor::new();
        assert!(!monitor.is_world_online("non-existent"));
    }

    #[test]
    fn test_get_all_online_worlds() {
        let monitor = HealthMonitor::new();
        monitor.record_heartbeat("world-1".to_string(), 10);
        monitor.record_heartbeat("world-2".to_string(), 20);

        let online = monitor.get_all_online_worlds();
        assert_eq!(online.len(), 2);
    }

    #[test]
    fn test_online_world_count() {
        let monitor = HealthMonitor::new();
        monitor.record_heartbeat("world-1".to_string(), 10);
        monitor.record_heartbeat("world-2".to_string(), 20);

        assert_eq!(monitor.online_world_count(), 2);
    }
}
