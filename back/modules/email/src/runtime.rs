use crate::{compose, sync};
use aphel_kernel::modules::{WorkerEnv, WorkerParts};

pub(crate) fn sending_worker(env: &WorkerEnv) -> WorkerParts {
    compose::runtime::worker(env.state())
}

pub(crate) fn sync_worker(env: &WorkerEnv) -> WorkerParts {
    sync::jobs::worker(env.state())
}
