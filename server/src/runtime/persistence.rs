use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use protocol::RouteKey;
use serde::Serialize;
use tokio::sync::{mpsc, oneshot, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterStateSnapshot {
    pub character_id: u64,
    pub route: RouteKey,
    pub x: u16,
    pub y: u16,
    pub hp: u16,
    pub mp: u16,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CriticalEventKind {
    TradeCommit,
    InventoryMutation,
    EconomyMutation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CriticalEvent {
    pub event_id: u128,
    pub character_id: u64,
    pub route: RouteKey,
    pub kind: CriticalEventKind,
    pub payload: String,
    pub occurred_at_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PersistenceMetrics {
    pub queue_depth: usize,
    pub pending_non_critical: usize,
    pub flush_count: u64,
    pub flushed_records: u64,
    pub critical_count: u64,
    pub error_count: u64,
    pub last_flush_duration_ms: u64,
}

impl Default for PersistenceMetrics {
    fn default() -> Self {
        Self {
            queue_depth: 0,
            pending_non_critical: 0,
            flush_count: 0,
            flushed_records: 0,
            critical_count: 0,
            error_count: 0,
            last_flush_duration_ms: 0,
        }
    }
}

#[derive(Debug)]
enum PersistenceCommand {
    UpsertNonCritical(CharacterStateSnapshot),
    FlushCharacter {
        character_id: u64,
    },
    RecordCritical {
        event: CriticalEvent,
        ack: Option<oneshot::Sender<Result<(), PersistenceError>>>,
    },
    FlushNow,
    Shutdown,
}

#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("persistence channel closed")]
    ChannelClosed,

    #[error("persistence sink error: {0}")]
    Sink(String),
}

pub trait PersistenceSink: Send + Sync + 'static {
    fn bulk_upsert_states(
        &self,
        states: Vec<CharacterStateSnapshot>,
    ) -> Result<(), PersistenceError>;
    fn write_critical_event(&self, event: CriticalEvent) -> Result<(), PersistenceError>;
}

#[derive(Clone)]
pub struct InMemoryPersistenceSink {
    states: Arc<DashMap<u64, CharacterStateSnapshot>>,
    critical_log: Arc<StdMutex<Vec<CriticalEvent>>>,
}

impl InMemoryPersistenceSink {
    pub fn new() -> Self {
        Self {
            states: Arc::new(DashMap::new()),
            critical_log: Arc::new(StdMutex::new(Vec::new())),
        }
    }

    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    pub fn critical_count(&self) -> usize {
        self.critical_log
            .lock()
            .map(|guard| guard.len())
            .unwrap_or_default()
    }

    pub fn get_state(&self, character_id: u64) -> Option<CharacterStateSnapshot> {
        self.states
            .get(&character_id)
            .map(|entry| entry.value().clone())
    }
}

impl Default for InMemoryPersistenceSink {
    fn default() -> Self {
        Self::new()
    }
}

impl PersistenceSink for InMemoryPersistenceSink {
    fn bulk_upsert_states(
        &self,
        states: Vec<CharacterStateSnapshot>,
    ) -> Result<(), PersistenceError> {
        for state in states {
            self.states.insert(state.character_id, state);
        }
        Ok(())
    }

    fn write_critical_event(&self, event: CriticalEvent) -> Result<(), PersistenceError> {
        let mut guard = self
            .critical_log
            .lock()
            .map_err(|e| PersistenceError::Sink(format!("mutex poisoned: {}", e)))?;
        guard.push(event);
        Ok(())
    }
}

#[derive(Clone)]
pub struct PersistenceHandle {
    tx: mpsc::Sender<PersistenceCommand>,
    metrics: Arc<Mutex<PersistenceMetrics>>,
}

impl PersistenceHandle {
    pub async fn enqueue_non_critical(
        &self,
        state: CharacterStateSnapshot,
    ) -> Result<(), PersistenceError> {
        self.tx
            .send(PersistenceCommand::UpsertNonCritical(state))
            .await
            .map_err(|_| PersistenceError::ChannelClosed)
    }

    pub async fn flush_character(&self, character_id: u64) -> Result<(), PersistenceError> {
        self.tx
            .send(PersistenceCommand::FlushCharacter { character_id })
            .await
            .map_err(|_| PersistenceError::ChannelClosed)
    }

    pub async fn record_critical(&self, event: CriticalEvent) -> Result<(), PersistenceError> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(PersistenceCommand::RecordCritical {
                event,
                ack: Some(ack_tx),
            })
            .await
            .map_err(|_| PersistenceError::ChannelClosed)?;

        ack_rx.await.map_err(|_| PersistenceError::ChannelClosed)?
    }

    pub async fn flush_now(&self) -> Result<(), PersistenceError> {
        self.tx
            .send(PersistenceCommand::FlushNow)
            .await
            .map_err(|_| PersistenceError::ChannelClosed)
    }

    pub async fn shutdown(&self) -> Result<(), PersistenceError> {
        self.tx
            .send(PersistenceCommand::Shutdown)
            .await
            .map_err(|_| PersistenceError::ChannelClosed)
    }

    pub async fn metrics(&self) -> PersistenceMetrics {
        self.metrics.lock().await.clone()
    }
}

pub fn start_persistence_worker(
    flush_tick: Duration,
    max_flush_lag: Duration,
    max_batch_size: usize,
    sink: Arc<dyn PersistenceSink>,
) -> PersistenceHandle {
    let (tx, mut rx) = mpsc::channel::<PersistenceCommand>(4096);
    let metrics = Arc::new(Mutex::new(PersistenceMetrics::default()));
    let metrics_clone = metrics.clone();

    tokio::spawn(async move {
        let mut pending: HashMap<u64, (CharacterStateSnapshot, Instant)> = HashMap::new();
        let mut tick = tokio::time::interval(flush_tick);

        loop {
            tokio::select! {
                maybe_cmd = rx.recv() => {
                    match maybe_cmd {
                        Some(PersistenceCommand::UpsertNonCritical(state)) => {
                            pending.insert(state.character_id, (state, Instant::now()));
                            let mut m = metrics_clone.lock().await;
                            m.queue_depth = rx.len();
                            m.pending_non_critical = pending.len();
                        }
                        Some(PersistenceCommand::FlushCharacter { character_id }) => {
                            if let Some((state, _)) = pending.remove(&character_id) {
                                if let Err(err) = sink.bulk_upsert_states(vec![state]) {
                                    let mut m = metrics_clone.lock().await;
                                    m.error_count += 1;
                                    log::error!("flush_character failed: {err}");
                                }
                            }
                        }
                        Some(PersistenceCommand::RecordCritical { event, ack }) => {
                            let result = sink.write_critical_event(event);
                            let mut m = metrics_clone.lock().await;
                            if result.is_ok() {
                                m.critical_count += 1;
                            } else {
                                m.error_count += 1;
                            }
                            if let Some(ack) = ack {
                                let _ = ack.send(result);
                            }
                        }
                        Some(PersistenceCommand::FlushNow) => {
                            flush_pending(&sink, &mut pending, max_batch_size, &metrics_clone).await;
                        }
                        Some(PersistenceCommand::Shutdown) | None => {
                            flush_pending(&sink, &mut pending, max_batch_size, &metrics_clone).await;
                            break;
                        }
                    }
                }
                _ = tick.tick() => {
                    flush_expired(&sink, &mut pending, max_flush_lag, max_batch_size, &metrics_clone).await;
                }
            }
        }
    });

    PersistenceHandle { tx, metrics }
}

async fn flush_expired(
    sink: &Arc<dyn PersistenceSink>,
    pending: &mut HashMap<u64, (CharacterStateSnapshot, Instant)>,
    max_flush_lag: Duration,
    max_batch_size: usize,
    metrics: &Arc<Mutex<PersistenceMetrics>>,
) {
    let now = Instant::now();
    let mut expired_ids = Vec::new();
    for (character_id, (_, inserted_at)) in pending.iter() {
        if now.duration_since(*inserted_at) >= max_flush_lag {
            expired_ids.push(*character_id);
        }
        if expired_ids.len() >= max_batch_size {
            break;
        }
    }

    if expired_ids.is_empty() && pending.len() >= max_batch_size {
        expired_ids.extend(pending.keys().copied().take(max_batch_size));
    }

    if expired_ids.is_empty() {
        let mut m = metrics.lock().await;
        m.pending_non_critical = pending.len();
        return;
    }

    let mut batch = Vec::with_capacity(expired_ids.len());
    for id in expired_ids {
        if let Some((snapshot, _)) = pending.remove(&id) {
            batch.push(snapshot);
        }
    }

    flush_batch(sink, batch, metrics).await;

    let mut m = metrics.lock().await;
    m.pending_non_critical = pending.len();
}

async fn flush_pending(
    sink: &Arc<dyn PersistenceSink>,
    pending: &mut HashMap<u64, (CharacterStateSnapshot, Instant)>,
    max_batch_size: usize,
    metrics: &Arc<Mutex<PersistenceMetrics>>,
) {
    while !pending.is_empty() {
        let mut batch = Vec::new();
        for key in pending
            .keys()
            .copied()
            .take(max_batch_size)
            .collect::<Vec<_>>()
        {
            if let Some((snapshot, _)) = pending.remove(&key) {
                batch.push(snapshot);
            }
        }

        flush_batch(sink, batch, metrics).await;
    }
}

async fn flush_batch(
    sink: &Arc<dyn PersistenceSink>,
    batch: Vec<CharacterStateSnapshot>,
    metrics: &Arc<Mutex<PersistenceMetrics>>,
) {
    if batch.is_empty() {
        return;
    }

    let started = Instant::now();
    let result = sink.bulk_upsert_states(batch.clone());

    let mut m = metrics.lock().await;
    m.flush_count += 1;
    m.last_flush_duration_ms = started.elapsed().as_millis() as u64;
    match result {
        Ok(()) => {
            m.flushed_records += batch.len() as u64;
        }
        Err(err) => {
            m.error_count += 1;
            log::error!("persistence flush failed: {err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_route() -> RouteKey {
        RouteKey {
            world_id: 1,
            entry_id: 1,
            map_id: 0,
            instance_id: 1,
        }
    }

    #[tokio::test]
    async fn coalesces_non_critical_states_and_flushes_last_snapshot() {
        let sink = Arc::new(InMemoryPersistenceSink::new());
        let handle = start_persistence_worker(
            Duration::from_millis(50),
            Duration::from_millis(50),
            100,
            sink.clone(),
        );

        handle
            .enqueue_non_critical(CharacterStateSnapshot {
                character_id: 10,
                route: sample_route(),
                x: 10,
                y: 20,
                hp: 90,
                mp: 50,
                updated_at_ms: 1,
            })
            .await
            .unwrap();

        handle
            .enqueue_non_critical(CharacterStateSnapshot {
                character_id: 10,
                route: sample_route(),
                x: 30,
                y: 40,
                hp: 80,
                mp: 45,
                updated_at_ms: 2,
            })
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(120)).await;

        let state = sink.get_state(10).expect("state must be flushed");
        assert_eq!(state.x, 30);
        assert_eq!(state.y, 40);

        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn critical_events_are_written_immediately() {
        let sink = Arc::new(InMemoryPersistenceSink::new());
        let handle = start_persistence_worker(
            Duration::from_secs(1),
            Duration::from_secs(10),
            100,
            sink.clone(),
        );

        handle
            .record_critical(CriticalEvent {
                event_id: 99,
                character_id: 7,
                route: sample_route(),
                kind: CriticalEventKind::TradeCommit,
                payload: "trade#99".to_string(),
                occurred_at_ms: 123,
            })
            .await
            .unwrap();

        assert_eq!(sink.critical_count(), 1);

        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn flush_character_forces_write() {
        let sink = Arc::new(InMemoryPersistenceSink::new());
        let handle = start_persistence_worker(
            Duration::from_secs(5),
            Duration::from_secs(10),
            100,
            sink.clone(),
        );

        handle
            .enqueue_non_critical(CharacterStateSnapshot {
                character_id: 77,
                route: sample_route(),
                x: 9,
                y: 9,
                hp: 100,
                mp: 100,
                updated_at_ms: 44,
            })
            .await
            .unwrap();

        handle.flush_character(77).await.unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;

        assert_eq!(sink.state_count(), 1);

        handle.shutdown().await.unwrap();
    }
}
