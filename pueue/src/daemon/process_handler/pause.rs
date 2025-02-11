use pueue_lib::{
    network::message::TaskSelection, process_helper::ProcessAction, settings::Settings,
    state::GroupStatus, task::TaskStatus,
};

use crate::{
    daemon::{
        process_handler::perform_action,
        state_helper::{save_state, LockedState},
    },
    internal_prelude::*,
    ok_or_shutdown,
};

/// Pause specific tasks or groups.
///
/// `wait` decides, whether running tasks will kept running until they finish on their own.
pub fn pause(settings: &Settings, state: &mut LockedState, selection: TaskSelection, wait: bool) {
    // Get the keys of all tasks that should be paused
    let keys: Vec<usize> = match selection {
        TaskSelection::TaskIds(task_ids) => task_ids,
        TaskSelection::Group(group_name) => {
            // Ensure that a given group exists. (Might not happen due to concurrency)
            let group = match state.groups.get_mut(&group_name) {
                Some(group) => group,
                None => return,
            };

            // Pause a specific group.
            group.status = GroupStatus::Paused;
            info!("Pausing group {group_name}");

            let filtered_tasks = state.filter_tasks_of_group(
                |task| matches!(task.status, TaskStatus::Running { .. }),
                &group_name,
            );

            filtered_tasks.matching_ids
        }
        TaskSelection::All => {
            // Pause all groups, since we're pausing the whole daemon.
            state.set_status_for_all_groups(GroupStatus::Paused);

            info!("Pausing everything");
            state.children.all_task_ids()
        }
    };

    // Pause all tasks that were found.
    if !wait {
        for id in keys {
            // Get the enqueued_at/start times from the current state.
            let (enqueued_at, start) = match state.tasks.get(&id).unwrap().status {
                TaskStatus::Running { enqueued_at, start }
                | TaskStatus::Paused { enqueued_at, start } => (enqueued_at, start),
                _ => continue,
            };

            let success = match perform_action(state, id, ProcessAction::Pause) {
                Err(err) => {
                    error!("Failed pausing task {id}: {err:?}");
                    false
                }
                Ok(success) => success,
            };

            if success {
                state.change_status(id, TaskStatus::Paused { enqueued_at, start });
            }
        }
    }

    ok_or_shutdown!(settings, state, save_state(state, settings));
}
