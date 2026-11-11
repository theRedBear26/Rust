
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::task::Task;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    ValueEnum,
    Serialize,
    Deserialize
)]
pub enum SortBy {
    Name,
    Priority,
    Deadline,
    Status,
}

pub fn sort_tasks(tasks: &mut [Task], sort_by: Option<SortBy>) {
    match sort_by {
        Some(SortBy::Name) => {
            tasks.sort_by(|a, b| {
                a.content.to_lowercase()
                    .cmp(&b.content.to_lowercase())
            });
        }

        Some(SortBy::Priority) => {
            tasks.sort_by_key(|task| match task.priority {
                crate::task::Priority::High => 0,
                crate::task::Priority::Medium => 1,
                crate::task::Priority::Low => 2,
            });
        }

        Some(SortBy::Deadline) => {
            tasks.sort_by(|a, b| {
                a.deadline.cmp(&b.deadline)
            });
        }

        Some(SortBy::Status) => {
            tasks.sort_by_key(|task| task.done);
        }

        None => {}
    }
}

pub fn search_tasks(tasks: &[Task], keyword: &str) -> Vec<Task> {
    let keyword = keyword.to_lowercase();

    tasks
        .iter()
        .filter(|task| {
            task.content.to_lowercase().contains(&keyword)
                || task.description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&keyword))
                    .unwrap_or(false)
        })
        .cloned()
        .collect()
}
