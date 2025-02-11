use pueue_lib::{network::message::*, state::SharedState, success_msg};

use crate::daemon::network::response_helper::*;

/// Set the parallel tasks for a specific group.
pub fn set_parallel_tasks(message: ParallelMessage, state: &SharedState) -> Response {
    let mut state = state.lock().unwrap();
    let group = match ensure_group_exists(&mut state, &message.group) {
        Ok(group) => group,
        Err(response) => return response,
    };

    group.parallel_tasks = message.parallel_tasks;

    success_msg!(
        "Parallel tasks setting for group \"{}\" adjusted",
        &message.group
    )
}
