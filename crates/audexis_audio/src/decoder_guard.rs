use std::panic::{AssertUnwindSafe, catch_unwind};

pub fn guard<T>(operation: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or_else(|payload| {
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("unknown decoder panic");
        Err(format!("Audio decoder panicked: {message}"))
    })
}
