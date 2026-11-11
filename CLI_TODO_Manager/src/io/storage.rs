
use std::cell::{Cell, RefCell};
use std::fs;
use std::path::PathBuf;

use crate::error::AppError;
use crate::task::{Priority, Task};
use crate::UserInterfaceTypes;

pub struct TaskStore {
    file_path: PathBuf,
    backup_path: PathBuf,
    tasks: RefCell<Vec<Task>>,
    notice: Cell<bool>,
}

impl TaskStore {
    pub fn new(
        file: Option<PathBuf>,
        _ui_type: UserInterfaceTypes,
    ) -> Self {
        let file_path = file.unwrap_or_else(|| {
            PathBuf::from("tasks.json")
        });

        let backup_path = PathBuf::from(
            format!("{}.bak", file_path.display())
        );

        let mut recovered = false;

        let tasks = if file_path.exists() {
            match fs::read_to_string(&file_path)
                .ok()
                .and_then(|data| {
                    serde_json::from_str::<Vec<Task>>(&data).ok()
                })
            {
                Some(tasks) => tasks,

                None => {
                    recovered = true;

                    fs::read_to_string(&backup_path)
                        .ok()
                        .and_then(|data| {
                            serde_json::from_str::<Vec<Task>>(&data).ok()
                        })
                        .unwrap_or_default()
                }
            }
        } else {
            Vec::new()
        };

        Self {
            file_path,
            backup_path,
            tasks: RefCell::new(tasks),
            notice: Cell::new(recovered),
        }
    }

    pub fn backup_path(&self) -> &PathBuf {
        &self.backup_path
    }

    pub fn take_notice(&self) -> Option<bool> {
        if self.notice.replace(false) {
            Some(true)
        } else {
            None
        }
    }

    pub fn all(&self) -> Vec<Task> {
        self.tasks.borrow().clone()
    }

    pub fn get(&self, no: usize) -> Result<Task, AppError> {
        if no == 0 {
            return Err(AppError::Message(
                "Task numbers start from 1.".to_string()
            ));
        }

        self.tasks
            .borrow()
            .get(no - 1)
            .cloned()
            .ok_or_else(|| {
                AppError::Message(format!(
                    "Task {} does not exist.",
                    no
                ))
            })
    }

    fn save(&self) -> Result<(), AppError> {
        if let Some(parent) = self.file_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        let data = serde_json::to_string_pretty(
            &*self.tasks.borrow()
        )?;

        fs::write(&self.file_path, data)?;

        Ok(())
    }

    pub fn add(&self, task: Task) -> Result<(), AppError> {
        self.tasks.borrow_mut().push(task);
        self.save()
    }

    pub fn update(
        &self,
        no: usize,
        content: Option<String>,
        description: Option<String>,
        deadline: Option<String>,
        priority: Option<Priority>,
    ) -> Result<(), AppError> {
        if no == 0 {
            return Err(AppError::Message(
                "Task numbers start from 1.".to_string()
            ));
        }

        {
            let mut tasks = self.tasks.borrow_mut();

            let task = tasks.get_mut(no - 1).ok_or_else(|| {
                AppError::Message(format!(
                    "Task {} does not exist.",
                    no
                ))
            })?;

            if let Some(value) = content {
                task.content = value;
            }

            if let Some(value) = description {
                task.description = Some(value);
            }

            if let Some(value) = deadline {
                task.deadline = Some(value);
            }

            if let Some(value) = priority {
                task.priority = value;
            }
        }

        self.save()
    }

    pub fn set_done(
        &self,
        no: usize,
        done: bool,
    ) -> Result<(), AppError> {
        if no == 0 {
            return Err(AppError::Message(
                "Task numbers start from 1.".to_string()
            ));
        }

        {
            let mut tasks = self.tasks.borrow_mut();

            let task = tasks.get_mut(no - 1).ok_or_else(|| {
                AppError::Message(format!(
                    "Task {} does not exist.",
                    no
                ))
            })?;

            task.done = done;
        }

        self.save()
    }

    pub fn delete(
        &self,
        numbers: &[usize],
        all_done: bool,
    ) -> Result<usize, AppError> {
        let current = self.tasks.borrow().clone();

        fs::write(
            &self.backup_path,
            serde_json::to_string_pretty(&current)?,
        )?;

        let mut tasks = self.tasks.borrow_mut();
        let original_len = tasks.len();

        if all_done {
            tasks.retain(|task| !task.done);
        } else {
            if numbers.is_empty() {
                return Err(AppError::Message(
                    "Provide task numbers to delete.".to_string()
                ));
            }

            for &no in numbers {
                if no == 0 || no > tasks.len() {
                    return Err(AppError::Message(format!(
                        "Invalid task number: {}",
                        no
                    )));
                }
            }

            let mut indexes: Vec<usize> = numbers
                .iter()
                .map(|no| no - 1)
                .collect();

            indexes.sort_unstable();
            indexes.dedup();

            for index in indexes.into_iter().rev() {
                tasks.remove(index);
            }
        }

        let deleted = original_len - tasks.len();

        drop(tasks);
        self.save()?;

        Ok(deleted)
    }

    pub fn undo(&self) -> Result<(), AppError> {
        if !self.backup_path.exists() {
            return Err(AppError::Message(
                "No backup available to undo.".to_string()
            ));
        }

        let data = fs::read_to_string(&self.backup_path)?;

        let restored: Vec<Task> =
            serde_json::from_str(&data)?;

        *self.tasks.borrow_mut() = restored;

        self.save()?;

        Ok(())
    }

    pub fn status(&self) -> (usize, usize, usize) {
        let tasks = self.tasks.borrow();

        let total = tasks.len();
        let completed = tasks.iter().filter(|t| t.done).count();
        let pending = total - completed;

        (total, completed, pending)
    }
}

pub fn recovered_from_backup_msg(
    path: &PathBuf,
    _ui_type: UserInterfaceTypes,
) -> String {
    format!(
        "Warning: Tasks were recovered from backup: {}",
        path.display()
    )
}
