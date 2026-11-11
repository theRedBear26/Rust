
use crate::error::AppError;
use crate::io::storage::TaskStore;
use crate::task::{Priority, Task};
use crate::time::{format_date, validate_date};
use crate::todo::{search_tasks, sort_tasks, SortBy};

pub fn add(
    content: String,
    store: &TaskStore,
    description: Option<String>,
    deadline: Option<String>,
    priority: Option<Priority>,
) -> Result<(), AppError> {
    if content.trim().is_empty() {
        return Err(AppError::Message(
            "Task content cannot be empty.".to_string()
        ));
    }

    if let Some(ref date) = deadline {
        validate_date(date).map_err(AppError::Message)?;
    }

    let task = Task::new(
        content,
        description,
        deadline,
        priority,
    );

    store.add(task)?;

    println!("Task added successfully!");

    Ok(())
}

pub fn change(
    no: usize,
    store: &TaskStore,
    content: Option<String>,
    description: Option<String>,
    deadline: Option<String>,
    priority: Option<Priority>,
) -> Result<(), AppError> {
    if let Some(ref date) = deadline {
        validate_date(date).map_err(AppError::Message)?;
    }

    store.update(
        no,
        content,
        description,
        deadline,
        priority,
    )?;

    println!("Task {} updated successfully!", no);

    Ok(())
}

pub fn list(
    store: &TaskStore,
    sort: Option<SortBy>,
    find: Option<String>,
) -> Result<(), AppError> {
    let mut tasks = store.all();

    if let Some(keyword) = find {
        tasks = search_tasks(&tasks, &keyword);
    }

    sort_tasks(&mut tasks, sort);

    if tasks.is_empty() {
        println!("No tasks found.");
        return Ok(());
    }

    println!("\n{:<5} {:<8} {:<12} TASK",
        "NO.", "STATUS", "PRIORITY"
    );

    println!("{}", "-".repeat(65));

    for (index, task) in tasks.iter().enumerate() {
        let status = if task.done {
            "Done"
        } else {
            "Pending"
        };

        println!(
            "{:<5} {:<8} {:<12} {}",
            index + 1,
            status,
            task.priority,
            task.content
        );

        if let Some(deadline) = &task.deadline {
            println!("      Deadline: {}", format_date(deadline));
        }
    }

    println!();

    Ok(())
}

pub fn show(
    no: usize,
    store: &TaskStore,
) -> Result<(), AppError> {
    let task = store.get(no)?;

    println!("\nTask #{}", no);
    println!("Content: {}", task.content);

    println!(
        "Description: {}",
        task.description.as_deref().unwrap_or("None")
    );

    println!(
        "Deadline: {}",
        task.deadline
            .as_deref()
            .map(format_date)
            .unwrap_or_else(|| "None".to_string())
    );

    println!("Priority: {}", task.priority);

    println!(
        "Status: {}",
        if task.done { "Completed" } else { "Pending" }
    );

    Ok(())
}

pub fn status(
    store: &TaskStore,
) -> Result<(), AppError> {
    let (total, completed, pending) = store.status();

    println!("TODO STATUS");
    println!("----------------");
    println!("Total tasks: {}", total);
    println!("Completed:   {}", completed);
    println!("Pending:     {}", pending);

    Ok(())
}

pub fn done(
    nos: Vec<usize>,
    store: &TaskStore,
) -> Result<(), AppError> {
    for no in nos {
        store.set_done(no, true)?;
        println!("Task {} marked as completed.", no);
    }

    Ok(())
}

pub fn undone(
    nos: Vec<usize>,
    store: &TaskStore,
) -> Result<(), AppError> {
    for no in nos {
        store.set_done(no, false)?;
        println!("Task {} marked as pending.", no);
    }

    Ok(())
}

pub fn undo(
    store: &TaskStore,
    yes: bool,
) -> Result<(), AppError> {
    if !yes {
        println!(
            "Use --yes to confirm restoring the previous backup."
        );
        return Ok(());
    }

    store.undo()?;

    println!("Previous task list restored successfully.");

    Ok(())
}

pub fn delete(
    nos: Vec<usize>,
    store: &TaskStore,
    alldone: bool,
    yes: bool,
) -> Result<(), AppError> {
    if !yes {
        println!("Use --yes to confirm deletion.");
        return Ok(());
    }

    let deleted = store.delete(&nos, alldone)?;

    println!("{} task(s) deleted successfully.", deleted);

    Ok(())
}
