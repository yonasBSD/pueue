//! The representation of the pueue daemon's current [State].
//! Contains all [`Task`]s and [`Group`]s of the daemon.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::task::Task;

pub const PUEUE_DEFAULT_GROUP: &str = "default";

/// Represents the current status of a group.
/// Each group acts as a queue and can be managed individually.
#[derive(PartialEq, Eq, Clone, Debug, Copy, Deserialize, Serialize)]
pub enum GroupStatus {
    Running,
    Paused,
    // This state is set, if this group is being reset.
    // This means that all tasks are being killed and removed.
    Reset,
}

/// The representation of a group.
#[derive(PartialEq, Eq, Clone, Debug, Deserialize, Serialize)]
pub struct Group {
    pub status: GroupStatus,
    pub parallel_tasks: usize,
}

/// This is the full representation of the current state of the Pueue daemon.
///
/// This includes
/// - The currently used settings.
/// - The full task list
/// - The current status of all tasks
/// - All known groups.
///
/// However, the State does NOT include:
/// - Information about child processes
/// - Handles to child processes
///
/// That information is saved in the daemon's TaskHandler.
///
/// Most functions implemented on the state shouldn't be used by third party software.
/// The daemon is constantly changing and persisting the state. \
/// Any changes applied to a state and saved to disk, will most likely be overwritten
/// after a short time.
///
///
/// The daemon uses the state as a piece of shared memory between it's threads.
/// It's wrapped in a MutexGuard, which allows us to guarantee sequential access to any crucial
/// information, such as status changes and incoming commands by the client.
#[derive(Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct State {
    /// All tasks currently managed by the daemon.
    pub tasks: BTreeMap<usize, Task>,
    /// All groups with their current state a configuration.
    pub groups: BTreeMap<String, Group>,
}

// Implement a custom Clone, as the child processes don't implement Clone.
impl Clone for State {
    fn clone(&self) -> Self {
        State {
            tasks: self.tasks.clone(),
            groups: self.groups.clone(),
        }
    }
}

/// A little helper struct that's returned by the state's task filter functions.
/// Contains all task ids of tasks that matched and didn't match a given condition.
#[derive(Debug, Default)]
pub struct FilteredTasks {
    pub matching_ids: Vec<usize>,
    pub non_matching_ids: Vec<usize>,
}

impl State {
    /// Create a new default state.
    pub fn new() -> State {
        Self::default()
    }

    /// Add a new task
    pub fn add_task(&mut self, mut task: Task) -> usize {
        let next_id = match self.tasks.keys().max() {
            None => 0,
            Some(id) => id + 1,
        };
        task.id = next_id;
        self.tasks.insert(next_id, task);

        next_id
    }

    /// Get all ids of task inside a specific group.
    pub fn task_ids_in_group(&self, group: &str) -> Vec<usize> {
        self.tasks
            .iter()
            .filter(|(_, task)| task.group.eq(group))
            .map(|(id, _)| *id)
            .collect()
    }

    /// This checks, whether some tasks match the expected filter criteria. \
    /// The first result is the list of task_ids that match these statuses. \
    /// The second result is the list of task_ids that don't match these statuses. \
    ///
    /// By default, this checks all tasks in the current state. If a list of task_ids is
    /// provided as the third parameter, only those tasks will be checked.
    pub fn filter_tasks<F>(&self, condition: F, task_ids: Option<Vec<usize>>) -> FilteredTasks
    where
        F: Fn(&Task) -> bool,
    {
        // Either use all tasks or only the explicitly specified ones.
        let task_ids = match task_ids {
            Some(ids) => ids,
            None => self.tasks.keys().cloned().collect(),
        };

        self.filter_task_ids(condition, task_ids)
    }

    /// Same as [State::filter_tasks], but only checks for tasks of a specific group.
    pub fn filter_tasks_of_group<F>(&self, condition: F, group: &str) -> FilteredTasks
    where
        F: Fn(&Task) -> bool,
    {
        // Return empty vectors, if there's no such group.
        if !self.groups.contains_key(group) {
            return FilteredTasks::default();
        }

        // Filter all task ids of tasks that match the given group.
        let task_ids = self
            .tasks
            .iter()
            .filter(|(_, task)| task.group == group)
            .map(|(id, _)| *id)
            .collect();

        self.filter_task_ids(condition, task_ids)
    }

    /// Internal function used to check which of the given tasks match the provided filter.
    ///
    /// Returns a tuple of all (matching_task_ids, non_matching_task_ids).
    fn filter_task_ids<F>(&self, condition: F, task_ids: Vec<usize>) -> FilteredTasks
    where
        F: Fn(&Task) -> bool,
    {
        let mut matching_ids = Vec::new();
        let mut non_matching_ids = Vec::new();

        // Filter all task id's that match the provided statuses.
        for task_id in task_ids.iter() {
            // Check whether the task exists and save all non-existing task ids.
            match self.tasks.get(task_id) {
                None => {
                    non_matching_ids.push(*task_id);
                    continue;
                }
                Some(task) => {
                    // Check whether the task status matches the filter.
                    if condition(task) {
                        matching_ids.push(*task_id);
                    } else {
                        non_matching_ids.push(*task_id);
                    }
                }
            };
        }

        FilteredTasks {
            matching_ids,
            non_matching_ids,
        }
    }
}
