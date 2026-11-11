
use crate::task::{Priority, Task};

#[test]
fn test_create_task() {
    let task = Task::new(
        "Learn Rust".to_string(),
        Some("Study Rust basics".to_string()),
        None,
        Some(Priority::High),
    );

    assert_eq!(task.content, "Learn Rust");
    assert_eq!(task.priority, Priority::High);
    assert!(!task.done);
}

#[test]
fn test_default_priority() {
    let task = Task::new(
        "Complete assignment".to_string(),
        None,
        None,
        None,
    );

    assert_eq!(task.priority, Priority::Medium);
}
