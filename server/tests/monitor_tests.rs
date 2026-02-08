use server::monitor::HealthMonitor;
use std::thread;
use std::time::Duration;

#[test]
fn test_health_monitor_creation() {
    let monitor = HealthMonitor::new();
    assert!(!monitor.is_world_online("any-world"));
}

#[test]
fn test_record_heartbeat() {
    let monitor = HealthMonitor::new();

    monitor.record_heartbeat("world-1".to_string(), 10);

    assert!(monitor.is_world_online("world-1"));
}

#[test]
fn test_multiple_heartbeats() {
    let monitor = HealthMonitor::new();

    monitor.record_heartbeat("world-1".to_string(), 10);
    monitor.record_heartbeat("world-2".to_string(), 20);
    monitor.record_heartbeat("world-3".to_string(), 30);

    assert!(monitor.is_world_online("world-1"));
    assert!(monitor.is_world_online("world-2"));
    assert!(monitor.is_world_online("world-3"));
}

#[test]
fn test_heartbeat_updates_existing_world() {
    let monitor = HealthMonitor::new();

    monitor.record_heartbeat("world-1".to_string(), 10);
    assert!(monitor.is_world_online("world-1"));

    // Send another heartbeat for the same world with different player count
    monitor.record_heartbeat("world-1".to_string(), 50);
    assert!(monitor.is_world_online("world-1"));

    // Verify player count was updated
    let (online, players) = monitor.get_world_status("world-1").unwrap();
    assert!(online);
    assert_eq!(players, 50);
}

#[test]
fn test_is_world_online() {
    let monitor = HealthMonitor::new();

    assert!(!monitor.is_world_online("world-1"));

    monitor.record_heartbeat("world-1".to_string(), 10);

    assert!(monitor.is_world_online("world-1"));
    assert!(!monitor.is_world_online("world-2"));
}

#[test]
fn test_cleanup_stale_heartbeats() {
    let monitor = HealthMonitor::new();

    // Note: This test would need to mock time or wait for actual timeout
    // For now, we just verify the method exists and can be called
    let removed = monitor.cleanup_stale_heartbeats();
    assert_eq!(removed, 0);
}

#[test]
fn test_concurrent_heartbeats() {
    use std::sync::Arc;

    let monitor = Arc::new(HealthMonitor::new());
    let mut handles = vec![];

    for i in 0..10 {
        let monitor_clone = Arc::clone(&monitor);
        let handle = thread::spawn(move || {
            let world_id = format!("world-{}", i);
            monitor_clone.record_heartbeat(world_id, i as u32 * 10);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all worlds are online
    for i in 0..10 {
        assert!(monitor.is_world_online(&format!("world-{}", i)));
    }
}

#[test]
fn test_heartbeat_timestamp_updates() {
    let monitor = HealthMonitor::new();

    monitor.record_heartbeat("world-1".to_string(), 10);

    // Wait a bit
    thread::sleep(Duration::from_millis(100));

    // Send another heartbeat
    monitor.record_heartbeat("world-1".to_string(), 20);

    // World should still be online
    assert!(monitor.is_world_online("world-1"));
}
