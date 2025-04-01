use std::thread;
use tracing::info;

// Temporary to observe the thread is not blocking
pub fn thread_logging(str: &str) {
    let thread_id = thread::current().id(); // Get thread ID
    info!("{}: {:?}", str, thread_id);
}
