use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::{debug, error};

use super::CoreEvent;

/// An event bus for publishing CoreEvents through async channels.
/// 
/// The EventBus provides a simple publish mechanism for events while
/// the actual subscription is handled by returning a Receiver from `new()`.
/// This design ensures a single consumer pattern for the event stream.
#[derive(Clone, Debug)]
pub struct EventBus {
    sender: Sender<CoreEvent>,
}

impl EventBus {
    /// Creates a new EventBus with the specified buffer size.
    /// 
    /// Returns a tuple of (EventBus, Receiver<CoreEvent>) where:
    /// - EventBus is used for publishing events
    /// - Receiver is used for consuming events
    /// 
    /// # Arguments
    /// 
    /// * `buffer_size` - The channel buffer size for backpressure handling
    /// 
    /// # Example
    /// 
    /// ```
    /// let (event_bus, mut receiver) = EventBus::new(100);
    /// 
    /// // Publish events
    /// event_bus.publish(CoreEvent::Error("Something went wrong".to_string())).await?;
    /// 
    /// // Receive events
    /// while let Some(event) = receiver.recv().await {
    ///     // Handle event
    /// }
    /// ```
    pub fn new(buffer_size: usize) -> (Self, Receiver<CoreEvent>) {
        let (sender, receiver) = mpsc::channel(buffer_size);
        (EventBus { sender }, receiver)
    }

    /// Publishes an event to all subscribers asynchronously.
    /// 
    /// # Arguments
    /// 
    /// * `event` - The CoreEvent to publish
    /// 
    /// # Returns
    /// 
    /// * `Ok(())` if the event was successfully sent
    /// * `Err(String)` if the channel is closed or the event couldn't be sent
    /// 
    /// # Example
    /// 
    /// ```
    /// let result = event_bus.publish(CoreEvent::ContainerListUpdate(containers)).await;
    /// if let Err(e) = result {
    ///     eprintln!("Failed to publish event: {}", e);
    /// }
    /// ```
    pub async fn publish(&self, event: CoreEvent) -> Result<(), String> {
        debug!("Publishing event: {:?}", event);
        self.sender
            .send(event)
            .await
            .map_err(|e| {
                error!("Failed to send event: {}", e);
                format!("Failed to send event: {}", e)
            })
    }

    /// Attempt to subscribe to events. 
    /// 
    /// Note: This returns an error as the current design provides the receiver
    /// directly from `new()`. This method exists for API compatibility.
    pub fn subscribe(&self) -> Result<Receiver<CoreEvent>, String> {
        Err("subscribe() is not supported - use the receiver from new() instead".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_basic_delivery() {
        let (bus, mut receiver) = EventBus::new(10);
        
        let event = CoreEvent::Error("Test error".to_string());
        bus.publish(event.clone()).await.unwrap();
        
        let received = receiver.recv().await.unwrap();
        match received {
            CoreEvent::Error(msg) => assert_eq!(msg, "Test error"),
            _ => panic!("Unexpected event type"),
        }
    }

    #[tokio::test]
    async fn test_event_bus_multiple_events() {
        let (bus, mut receiver) = EventBus::new(10);
        
        let events = vec![
            CoreEvent::Error("Error 1".to_string()),
            CoreEvent::Error("Error 2".to_string()),
            CoreEvent::ContainerRemoved("container123".to_string()),
        ];
        
        for event in &events {
            bus.publish(event.clone()).await.unwrap();
        }
        
        for expected in events {
            let received = receiver.recv().await.unwrap();
            match (expected, received) {
                (CoreEvent::Error(e1), CoreEvent::Error(e2)) => assert_eq!(e1, e2),
                (CoreEvent::ContainerRemoved(c1), CoreEvent::ContainerRemoved(c2)) => assert_eq!(c1, c2),
                _ => panic!("Event mismatch"),
            }
        }
    }
}