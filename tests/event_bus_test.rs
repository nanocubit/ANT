//! Тесты для EventBus

mod common;

#[cfg(test)]
mod tests {
    use ant::bus::{EventBus, SystemEvent};
    use std::sync::Arc;
    use tokio::time::{timeout, Duration};

    #[test]
    fn test_event_bus_creation() {
        let bus = EventBus::new();
        assert!(true, "EventBus should be created");
    }

    #[tokio::test]
    async fn test_event_bus_emit_and_receive() {
        let bus = Arc::new(EventBus::new());
        let mut rx = bus.subscribe();

        let event = SystemEvent::GoalCreated {
            id: "test-1".into(),
            task: "Test task".into(),
        };

        bus.emit(event.clone());

        let received = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("Timeout")
            .expect("Channel closed");

        match received {
            SystemEvent::GoalCreated { id, task } => {
                assert_eq!(id, "test-1");
                assert_eq!(task, "Test task");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_event_bus_multiple_subscribers() {
        let bus = Arc::new(EventBus::new());
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let event = SystemEvent::SystemBoot("v0.9.0".into());
        bus.emit(event);

        let (r1, r2) = tokio::join!(
            timeout(Duration::from_secs(1), rx1.recv()),
            timeout(Duration::from_secs(1), rx2.recv())
        );

        assert!(r1.unwrap().is_ok());
        assert!(r2.unwrap().is_ok());
    }

    #[tokio::test]
    async fn test_event_bus_system_boot() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();

        bus.emit(SystemEvent::SystemBoot("test-version".into()));

        let event = timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("Timeout")
            .expect("Channel closed");

        match event {
            SystemEvent::SystemBoot(version) => {
                assert_eq!(version, "test-version");
            }
            _ => panic!("Expected SystemBoot event"),
        }
    }

    #[tokio::test]
    async fn test_event_bus_task_lifecycle() {
        let bus = Arc::new(EventBus::new());
        let mut rx = bus.subscribe();

        // Dispatch
        bus.emit(SystemEvent::TaskDispatched {
            task_id: "task-1".into(),
            tool: "test-tool".into(),
            input: "test-input".into(),
        });

        // Complete
        bus.emit(SystemEvent::TaskCompleted {
            task_id: "task-1".into(),
            result: "success".into(),
        });

        let events: Vec<SystemEvent> = timeout(
            Duration::from_secs(1),
            async {
                let mut events = Vec::new();
                for _ in 0..2 {
                    if let Ok(ev) = rx.recv().await {
                        events.push(ev);
                    }
                }
                events
            },
        )
        .await
        .expect("Timeout");

        assert_eq!(events.len(), 2);

        match &events[0] {
            SystemEvent::TaskDispatched { task_id, .. } => {
                assert_eq!(task_id, "task-1");
            }
            _ => panic!("First event should be TaskDispatched"),
        }

        match &events[1] {
            SystemEvent::TaskCompleted { task_id, .. } => {
                assert_eq!(task_id, "task-1");
            }
            _ => panic!("Second event should be TaskCompleted"),
        }
    }

    #[test]
    fn test_system_event_names() {
        assert_eq!(
            SystemEvent::SystemBoot("".into()).name(),
            "SystemBoot"
        );
        assert_eq!(
            SystemEvent::GoalCreated { id: "".into(), task: "".into() }.name(),
            "GoalCreated"
        );
        assert_eq!(
            SystemEvent::TaskCompleted { task_id: "".into(), result: "".into() }.name(),
            "TaskCompleted"
        );
        assert_eq!(
            SystemEvent::TaskFailed { task_id: "".into(), error: "".into() }.name(),
            "TaskFailed"
        );
    }
}
