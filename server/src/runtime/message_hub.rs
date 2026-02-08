use std::sync::Arc;

use dashmap::DashMap;
use protocol::{ChatPayload, RouteKey};
use tokio::sync::broadcast;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageScope {
    LocalMap(RouteKey),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HubMessage {
    pub from_session_id: u64,
    pub route: RouteKey,
    pub payload: ChatPayload,
}

#[derive(Clone)]
pub struct MessageHub {
    channels: Arc<DashMap<String, broadcast::Sender<HubMessage>>>,
    channel_capacity: usize,
}

impl MessageHub {
    pub fn new(channel_capacity: usize) -> Self {
        Self {
            channels: Arc::new(DashMap::new()),
            channel_capacity,
        }
    }

    #[cfg(test)]
    pub fn subscribe(&self, scope: MessageScope) -> broadcast::Receiver<HubMessage> {
        let key = scope_key(&scope);
        let sender = self.channels.entry(key).or_insert_with(|| {
            let (tx, _) = broadcast::channel(self.channel_capacity);
            tx
        });

        sender.subscribe()
    }

    pub fn publish(&self, scope: MessageScope, message: HubMessage) -> usize {
        let key = scope_key(&scope);
        let sender = self.channels.entry(key).or_insert_with(|| {
            let (tx, _) = broadcast::channel(self.channel_capacity);
            tx
        });

        sender.send(message).unwrap_or(0)
    }
}

impl Default for MessageHub {
    fn default() -> Self {
        Self::new(1024)
    }
}

fn scope_key(scope: &MessageScope) -> String {
    match scope {
        MessageScope::LocalMap(route) => format!(
            "local:{}:{}:{}:{}",
            route.world_id, route.entry_id, route.map_id, route.instance_id
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn publish_and_subscribe_local_map() {
        let hub = MessageHub::new(8);
        let route = RouteKey {
            world_id: 1,
            entry_id: 1,
            map_id: 0,
            instance_id: 1,
        };

        let mut rx = hub.subscribe(MessageScope::LocalMap(route));
        let delivered = hub.publish(
            MessageScope::LocalMap(route),
            HubMessage {
                from_session_id: 7,
                route,
                payload: ChatPayload {
                    channel: protocol::ChatChannel::Local,
                    target: None,
                    text: "hello".to_string(),
                },
            },
        );

        assert_eq!(delivered, 1);
        let msg = rx.recv().await.expect("must receive message");
        assert_eq!(msg.payload.text, "hello");
    }
}
