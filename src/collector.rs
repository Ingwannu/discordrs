#[cfg(feature = "collectors")]
use std::sync::Arc;
#[cfg(feature = "collectors")]
use std::time::Duration;

#[cfg(feature = "collectors")]
use tokio::sync::broadcast;
#[cfg(feature = "collectors")]
use tokio::time;

#[cfg(feature = "collectors")]
use crate::event::Event;
#[cfg(feature = "collectors")]
use crate::model::{ComponentInteraction, Interaction, Message, ModalSubmitInteraction};

#[cfg(feature = "collectors")]
type EventFilter<T> = Arc<dyn Fn(&T) -> bool + Send + Sync>;

#[cfg(feature = "collectors")]
#[derive(Clone)]
pub struct CollectorHub {
    sender: broadcast::Sender<Event>,
}

#[cfg(feature = "collectors")]
impl Default for CollectorHub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "collectors")]
impl CollectorHub {
    pub fn new() -> Self {
        Self::with_capacity(256)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity.max(1));
        Self { sender }
    }

    pub(crate) fn publish(&self, event: Event) {
        let _ = self.sender.send(event);
    }

    pub fn message_collector(&self) -> MessageCollector {
        MessageCollector::new(self.sender.subscribe())
    }

    pub fn interaction_collector(&self) -> InteractionCollector {
        InteractionCollector::new(self.sender.subscribe())
    }

    pub fn component_collector(&self) -> ComponentCollector {
        ComponentCollector::new(self.sender.subscribe())
    }

    pub fn modal_collector(&self) -> ModalCollector {
        ModalCollector::new(self.sender.subscribe())
    }
}

#[cfg(feature = "collectors")]
pub struct MessageCollector {
    receiver: broadcast::Receiver<Event>,
    filter: Option<EventFilter<Message>>,
    timeout: Option<Duration>,
    deadline: Option<time::Instant>,
    max_items: Option<usize>,
    lagged_events: u64,
}

#[cfg(feature = "collectors")]
impl MessageCollector {
    fn new(receiver: broadcast::Receiver<Event>) -> Self {
        Self {
            receiver,
            filter: None,
            timeout: None,
            deadline: None,
            max_items: None,
            lagged_events: 0,
        }
    }

    pub fn filter<F>(mut self, filter: F) -> Self
    where
        F: Fn(&Message) -> bool + Send + Sync + 'static,
    {
        self.filter = Some(Arc::new(filter));
        self
    }

    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    pub fn max_items(mut self, max_items: usize) -> Self {
        self.max_items = Some(max_items);
        self
    }

    pub fn lagged_events(&self) -> u64 {
        self.lagged_events
    }

    pub async fn next(&mut self) -> Option<Message> {
        let timeout = remaining_timeout(self.timeout, &mut self.deadline);
        recv_with_timeout(timeout, async {
            loop {
                match self.receiver.recv().await {
                    Ok(Event::MessageCreate(event)) | Ok(Event::MessageUpdate(event)) => {
                        let passes = self
                            .filter
                            .as_ref()
                            .map(|filter| filter(&event.message))
                            .unwrap_or(true);
                        if passes {
                            return Some(event.message);
                        }
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Closed) => return None,
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        self.lagged_events = self.lagged_events.saturating_add(skipped);
                    }
                }
            }
        })
        .await
    }

    pub async fn collect(mut self) -> Vec<Message> {
        let mut messages = Vec::new();
        while let Some(message) = self.next().await {
            messages.push(message);
            if let Some(max_items) = self.max_items {
                if messages.len() >= max_items {
                    break;
                }
            }
        }
        messages
    }
}

#[cfg(feature = "collectors")]
pub struct InteractionCollector {
    receiver: broadcast::Receiver<Event>,
    filter: Option<EventFilter<Interaction>>,
    timeout: Option<Duration>,
    deadline: Option<time::Instant>,
    max_items: Option<usize>,
    lagged_events: u64,
}

#[cfg(feature = "collectors")]
impl InteractionCollector {
    fn new(receiver: broadcast::Receiver<Event>) -> Self {
        Self {
            receiver,
            filter: None,
            timeout: None,
            deadline: None,
            max_items: None,
            lagged_events: 0,
        }
    }

    pub fn filter<F>(mut self, filter: F) -> Self
    where
        F: Fn(&Interaction) -> bool + Send + Sync + 'static,
    {
        self.filter = Some(Arc::new(filter));
        self
    }

    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    pub fn max_items(mut self, max_items: usize) -> Self {
        self.max_items = Some(max_items);
        self
    }

    pub fn lagged_events(&self) -> u64 {
        self.lagged_events
    }

    pub async fn next(&mut self) -> Option<Interaction> {
        let timeout = remaining_timeout(self.timeout, &mut self.deadline);
        recv_with_timeout(timeout, async {
            loop {
                match self.receiver.recv().await {
                    Ok(Event::InteractionCreate(event)) => {
                        let passes = self
                            .filter
                            .as_ref()
                            .map(|filter| filter(&event.interaction))
                            .unwrap_or(true);
                        if passes {
                            return Some(event.interaction);
                        }
                    }
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Closed) => return None,
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        self.lagged_events = self.lagged_events.saturating_add(skipped);
                    }
                }
            }
        })
        .await
    }

    pub async fn collect(mut self) -> Vec<Interaction> {
        let mut interactions = Vec::new();
        while let Some(interaction) = self.next().await {
            interactions.push(interaction);
            if let Some(max_items) = self.max_items {
                if interactions.len() >= max_items {
                    break;
                }
            }
        }
        interactions
    }
}

#[cfg(feature = "collectors")]
pub struct ComponentCollector {
    inner: InteractionCollector,
}

#[cfg(feature = "collectors")]
impl ComponentCollector {
    fn new(receiver: broadcast::Receiver<Event>) -> Self {
        Self {
            inner: InteractionCollector::new(receiver),
        }
    }

    pub fn timeout(mut self, duration: Duration) -> Self {
        self.inner = self.inner.timeout(duration);
        self
    }

    pub fn lagged_events(&self) -> u64 {
        self.inner.lagged_events()
    }

    pub fn max_items(mut self, max_items: usize) -> Self {
        self.inner = self.inner.max_items(max_items);
        self
    }

    pub async fn next(&mut self) -> Option<ComponentInteraction> {
        while let Some(interaction) = self.inner.next().await {
            if let Interaction::Component(component) = interaction {
                return Some(component);
            }
        }
        None
    }

    pub async fn collect(mut self) -> Vec<ComponentInteraction> {
        let mut components = Vec::new();
        while let Some(component) = self.next().await {
            components.push(component);
            if let Some(max_items) = self.inner.max_items {
                if components.len() >= max_items {
                    break;
                }
            }
        }
        components
    }
}

#[cfg(feature = "collectors")]
pub struct ModalCollector {
    inner: InteractionCollector,
}

#[cfg(feature = "collectors")]
impl ModalCollector {
    fn new(receiver: broadcast::Receiver<Event>) -> Self {
        Self {
            inner: InteractionCollector::new(receiver),
        }
    }

    pub fn timeout(mut self, duration: Duration) -> Self {
        self.inner = self.inner.timeout(duration);
        self
    }

    pub fn lagged_events(&self) -> u64 {
        self.inner.lagged_events()
    }

    pub fn max_items(mut self, max_items: usize) -> Self {
        self.inner = self.inner.max_items(max_items);
        self
    }

    pub async fn next(&mut self) -> Option<ModalSubmitInteraction> {
        while let Some(interaction) = self.inner.next().await {
            if let Interaction::ModalSubmit(modal) = interaction {
                return Some(modal);
            }
        }
        None
    }

    pub async fn collect(mut self) -> Vec<ModalSubmitInteraction> {
        let mut modals = Vec::new();
        while let Some(modal) = self.next().await {
            modals.push(modal);
            if let Some(max_items) = self.inner.max_items {
                if modals.len() >= max_items {
                    break;
                }
            }
        }
        modals
    }
}

#[cfg(feature = "collectors")]
fn remaining_timeout(
    timeout_duration: Option<Duration>,
    deadline: &mut Option<time::Instant>,
) -> Option<Duration> {
    let duration = timeout_duration?;
    let now = time::Instant::now();
    let deadline = *deadline.get_or_insert(now + duration);
    Some(deadline.saturating_duration_since(now))
}

#[cfg(feature = "collectors")]
async fn recv_with_timeout<T>(
    timeout_duration: Option<Duration>,
    future: impl std::future::Future<Output = Option<T>>,
) -> Option<T> {
    match timeout_duration {
        Some(duration) => time::timeout(duration, future).await.ok().flatten(),
        None => future.await,
    }
}

#[cfg(all(test, feature = "collectors"))]
mod tests {
    use std::time::Duration;

    use serde_json::json;
    use serde_json::Value;
    use tokio::time;

    use crate::event::decode_event;
    use crate::model::Interaction;

    use super::{recv_with_timeout, CollectorHub};

    fn interaction_event(payload: Value) -> crate::event::Event {
        decode_event("INTERACTION_CREATE", payload).expect("valid interaction event")
    }

    fn ping_interaction(id: &str) -> crate::event::Event {
        interaction_event(json!({
            "id": id,
            "application_id": "2",
            "token": "token",
            "type": 1
        }))
    }

    fn component_interaction(id: &str, custom_id: &str) -> crate::event::Event {
        interaction_event(json!({
            "id": id,
            "application_id": "2",
            "token": "token",
            "type": 3,
            "data": {
                "custom_id": custom_id,
                "component_type": 2,
                "values": ["one"]
            }
        }))
    }

    fn modal_interaction(id: &str, custom_id: &str) -> crate::event::Event {
        interaction_event(json!({
            "id": id,
            "application_id": "2",
            "token": "token",
            "type": 5,
            "data": {
                "custom_id": custom_id,
                "components": []
            }
        }))
    }

    #[tokio::test]
    async fn message_collector_stops_after_max_items() {
        let hub = CollectorHub::new();
        let collector = hub
            .message_collector()
            .max_items(1)
            .timeout(Duration::from_secs(1));

        hub.publish(
            decode_event(
                "MESSAGE_CREATE",
                json!({
                    "id": "2",
                    "channel_id": "1",
                    "content": "hello",
                    "mentions": [],
                    "attachments": []
                }),
            )
            .unwrap(),
        );

        let messages = collector.collect().await;
        assert_eq!(messages.len(), 1);
    }

    #[tokio::test]
    async fn message_collector_reports_lagged_events() {
        let hub = CollectorHub::with_capacity(1);
        let mut collector = hub.message_collector().timeout(Duration::from_secs(1));

        for id in ["2", "3", "4"] {
            hub.publish(
                decode_event(
                    "MESSAGE_CREATE",
                    json!({
                        "id": id,
                        "channel_id": "1",
                        "content": format!("message-{id}"),
                        "mentions": [],
                        "attachments": []
                    }),
                )
                .unwrap(),
            );
        }

        let message = collector
            .next()
            .await
            .expect("collector should still yield the newest buffered message");
        assert_eq!(message.id.as_str(), "4");
        assert!(collector.lagged_events() >= 1);
    }

    #[tokio::test]
    async fn interaction_component_and_modal_collectors_yield_typed_variants() {
        let hub = CollectorHub::new();
        let mut interaction_collector = hub
            .interaction_collector()
            .filter(|interaction| matches!(interaction, Interaction::Component(_)))
            .timeout(Duration::from_secs(1));
        let mut component_collector = hub.component_collector().timeout(Duration::from_secs(1));
        let mut modal_collector = hub.modal_collector().timeout(Duration::from_secs(1));

        hub.publish(
            decode_event(
                "INTERACTION_CREATE",
                json!({
                    "id": "1",
                    "application_id": "2",
                    "token": "token",
                    "type": 1
                }),
            )
            .unwrap(),
        );
        hub.publish(
            decode_event(
                "INTERACTION_CREATE",
                json!({
                    "id": "3",
                    "application_id": "4",
                    "token": "token",
                    "type": 3,
                    "data": {
                        "custom_id": "button",
                        "component_type": 2,
                        "values": ["one"]
                    }
                }),
            )
            .unwrap(),
        );
        hub.publish(
            decode_event(
                "INTERACTION_CREATE",
                json!({
                    "id": "5",
                    "application_id": "6",
                    "token": "token",
                    "type": 5,
                    "data": {
                        "custom_id": "modal",
                        "components": []
                    }
                }),
            )
            .unwrap(),
        );

        match interaction_collector.next().await {
            Some(Interaction::Component(component)) => {
                assert_eq!(component.data.custom_id, "button");
            }
            other => panic!("unexpected filtered interaction: {other:?}"),
        }

        let component = component_collector
            .next()
            .await
            .expect("component interaction");
        assert_eq!(component.context.id.as_str(), "3");
        assert_eq!(component.context.application_id.as_str(), "4");
        assert_eq!(component.context.token, "token");
        assert_eq!(component.data.custom_id, "button");
        assert_eq!(component.data.component_type, 2);
        assert_eq!(component.data.values, vec!["one".to_string()]);

        let modal = modal_collector.next().await.expect("modal interaction");
        assert_eq!(modal.context.id.as_str(), "5");
        assert_eq!(modal.submission.custom_id, "modal");
    }

    #[tokio::test]
    async fn interaction_collector_filter_skips_non_matches_and_max_items_chain_is_usable() {
        let hub = CollectorHub::new();
        let mut collector = hub
            .interaction_collector()
            .max_items(1)
            .filter(|interaction| matches!(interaction, Interaction::Component(_)))
            .timeout(Duration::from_secs(1));

        hub.publish(ping_interaction("1"));
        hub.publish(component_interaction("2", "keep-me"));

        match collector.next().await {
            Some(Interaction::Component(component)) => {
                assert_eq!(component.context.id.as_str(), "2");
                assert_eq!(component.data.custom_id, "keep-me");
            }
            other => panic!("unexpected interaction after filter skip: {other:?}"),
        }
    }

    #[tokio::test]
    async fn interaction_collector_collect_stops_at_max_items() {
        let hub = CollectorHub::new();
        let collector = hub
            .interaction_collector()
            .max_items(2)
            .timeout(Duration::from_secs(1));

        hub.publish(component_interaction("1", "first"));
        hub.publish(component_interaction("2", "second"));
        hub.publish(component_interaction("3", "third"));

        let interactions = collector.collect().await;
        assert_eq!(interactions.len(), 2);
        assert_eq!(interactions[0].id().as_str(), "1");
        assert_eq!(interactions[1].id().as_str(), "2");
    }

    #[tokio::test]
    async fn component_collector_timeout_is_absolute_across_non_matching_interactions() {
        let hub = CollectorHub::new();
        let mut collector = hub.component_collector().timeout(Duration::from_millis(30));
        let publisher = hub.clone();

        tokio::spawn(async move {
            for id in 0..20 {
                publisher.publish(ping_interaction(&id.to_string()));
                time::sleep(Duration::from_millis(10)).await;
            }
        });

        let result = time::timeout(Duration::from_millis(120), collector.next())
            .await
            .expect("collector timeout should not reset for every non-component interaction");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn interaction_collector_reports_lagged_interactions() {
        let hub = CollectorHub::with_capacity(1);
        let mut collector = hub.interaction_collector().timeout(Duration::from_secs(1));

        hub.publish(ping_interaction("1"));
        hub.publish(component_interaction("2", "older"));
        hub.publish(modal_interaction("3", "latest"));

        let interaction = collector
            .next()
            .await
            .expect("collector should yield the newest buffered interaction");
        match interaction {
            Interaction::ModalSubmit(modal) => {
                assert_eq!(modal.context.id.as_str(), "3");
                assert_eq!(modal.submission.custom_id, "latest");
            }
            other => panic!("unexpected interaction after lag: {other:?}"),
        }
        assert!(collector.lagged_events() >= 1);
    }

    #[tokio::test]
    async fn component_collector_returns_none_after_only_non_component_events() {
        let hub = CollectorHub::new();
        let mut collector = hub.component_collector().timeout(Duration::from_secs(1));

        hub.publish(ping_interaction("1"));
        hub.publish(modal_interaction("2", "not-a-component"));
        drop(hub);

        assert!(collector.next().await.is_none());
    }

    #[tokio::test]
    async fn component_collector_times_out_when_no_component_arrives() {
        let hub = CollectorHub::new();
        let mut collector = hub.component_collector().timeout(Duration::from_millis(20));

        hub.publish(ping_interaction("1"));

        assert!(collector.next().await.is_none());
        assert_eq!(collector.lagged_events(), 0);
    }

    #[tokio::test]
    async fn modal_collector_returns_none_after_only_non_modal_events() {
        let hub = CollectorHub::new();
        let mut collector = hub.modal_collector().timeout(Duration::from_secs(1));

        hub.publish(ping_interaction("1"));
        hub.publish(component_interaction("2", "button"));
        drop(hub);

        assert!(collector.next().await.is_none());
    }

    #[tokio::test]
    async fn modal_collector_times_out_when_no_modal_arrives() {
        let hub = CollectorHub::new();
        let mut collector = hub.modal_collector().timeout(Duration::from_millis(20));

        hub.publish(component_interaction("1", "button"));

        assert!(collector.next().await.is_none());
        assert_eq!(collector.lagged_events(), 0);
    }

    #[tokio::test]
    async fn recv_with_timeout_returns_none_when_future_exceeds_deadline() {
        let timed_out = recv_with_timeout(Some(Duration::from_millis(5)), async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            Some(1_u8)
        })
        .await;
        assert_eq!(timed_out, None);

        let completed = recv_with_timeout(None, async { Some(2_u8) }).await;
        assert_eq!(completed, Some(2));
    }
}
