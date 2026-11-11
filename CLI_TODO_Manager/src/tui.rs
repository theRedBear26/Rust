
use std::io::{self, Write};

use crate::commands;
use crate::error::AppError;
use crate::io::storage::TaskStore;
use crate::task::Priority;
use crate::todo::SortBy;

pub fn run(store: &TaskStore) -> Result<(), AppError> {
    loop {
        println!("\n================================");
        println!("       RUST TODO MANAGER");
        println!("================================");
        println!("1. Add task");
        println!("2. List tasks");
        println!("3. Show task");
        println!("4. Mark task as done");
        println!("5. Mark task as undone");
        println!("6. Change task");
        println!("7. Delete task");
        println!("8. Delete all completed tasks");
        println!("9. Show task status");
        println!("10. Undo deletion");
        println!("0. Exit");
        println!("================================");

        let choice = read_input("Choose an option: ");

        match choice.trim() {
            "1" => {
                let content = read_input("Task content: ");

                let description = read_input(
                    "Description (leave empty to skip): "
                );

                let deadline = read_input(
                    "Deadline YYYY-MM-DD (leave empty to skip): "
                );

                let priority = read_input(
                    "Priority (low/medium/high, default medium): "
                );

                let priority = match priority.to_lowercase().trim() {
                    "low" => Some(Priority::Low),
                    "high" => Some(Priority::High),
                    "medium" | "" => Some(Priority::Medium),
                    _ => {
                        println!("Invalid priority. Using medium.");
                        Some(Priority::Medium)
                    }
                };

                commands::add(
                    content,
                    store,
                    if description.trim().is_empty() {
                        None
                    } else {
                        Some(description)
                    },
                    if deadline.trim().is_empty() {
                        None
                    } else {
                        Some(deadline)
                    },
                    priority,
                )?;
            }

            "2" => {
                let sort = read_input(
                    "Sort by (name/priority/deadline/status/none): "
                );

                let sort = match sort.to_lowercase().trim() {
                    "name" => Some(SortBy::Name),
                    "priority" => Some(SortBy::Priority),
                    "deadline" => Some(SortBy::Deadline),
                    "status" => Some(SortBy::Status),
                    _ => None,
                };

                let find = read_input(
                    "Search keyword (leave empty for all): "
                );

                commands::list(
                    store,
                    sort,
                    if find.trim().is_empty() {
                        None
                    } else {
                        Some(find)
                    },
                )?;
            }

            "3" => {
                let no = read_number("Task number: ")?;
                commands::show(no, store)?;
            }

            "4" => {
                let no = read_number("Task number: ")?;
                commands::done(vec![no], store)?;
            }

            "5" => {
                let no = read_number("Task number: ")?;
                commands::undone(vec![no], store)?;
            }

            "6" => {
                let no = read_number("Task number: ")?;

                let content = read_input(
                    "New content (empty to keep current): "
                );

                let description = read_input(
                    "New description (empty to keep current): "
                );

                let deadline = read_input(
                    "New deadline YYYY-MM-DD (empty to keep current): "
                );

                let priority = read_input(
                    "New priority (low/medium/high, empty to keep current): "
                );

                let priority = match priority.to_lowercase().trim() {
                    "low" => Some(Priority::Low),
                    "medium" => Some(Priority::Medium),
                    "high" => Some(Priority::High),
                    _ => None,
                };

                commands::change(
                    no,
                    store,
                    if content.trim().is_empty() {
                        None
                    } else {
                        Some(content)
                    },
                    if description.trim().is_empty() {
                        None
                    } else {
                        Some(description)
                    },
                    if deadline.trim().is_empty() {
                        None
                    } else {
                        Some(deadline)
                    },
                    priority,
                )?;
            }

            "7" => {
                let no = read_number("Task number to delete: ")?;

                let confirm = read_input(
                    "Are you sure? (y/n): "
                );

                if confirm.to_lowercase().trim() == "y" {
                    commands::delete(
                        vec![no],
                        store,
                        false,
                        true,
                    )?;
                }
            }

            "8" => {
                let confirm = read_input(
                    "Delete all completed tasks? (y/n): "
                );

                if confirm.to_lowercase().trim() == "y" {
                    commands::delete(
                        vec![],
                        store,
                        true,
                        true,
                    )?;
                }
            }

            "9" => {
                commands::status(store)?;
            }

            "10" => {
                let confirm = read_input(
                    "Restore previous task list? (y/n): "
                );

                if confirm.to_lowercase().trim() == "y" {
                    commands::undo(store, true)?;
                }
            }

            "0" => {
                println!("Goodbye!");
                break;
            }

            _ => {
                println!("Invalid option. Please try again.");
            }
        }
    }

    Ok(())
}

fn read_input(prompt: &str) -> String {
    print!("{}", prompt);
    let _ = io::stdout().flush();

    let mut input = String::new();

    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");

    input.trim().to_string()
}

fn read_number(prompt: &str) -> Result<usize, AppError> {
    let input = read_input(prompt);

    input.parse::<usize>().map_err(|_| {
        AppError::Message(
            "Please enter a valid task number.".to_string()
        )
    })
}
