use freya::prelude::*;
use std::future::Future;

#[derive(Clone)]
pub struct InfinitePages<T> {
    pub pages: Vec<T>,
    pub next: Option<i64>,
    pub loading: bool,
    pub error: Option<String>,
}
impl<T> Default for InfinitePages<T> {
    fn default() -> Self {
        Self {
            pages: vec![],
            next: None,
            loading: false,
            error: None,
        }
    }
}
impl<T> InfinitePages<T> {
    pub fn has_more(&self) -> bool {
        self.pages.is_empty() || self.next.is_some()
    }
    fn complete(&mut self, result: Result<(T, Option<i64>), String>) {
        self.loading = false;
        match result {
            Ok((page, next)) => {
                self.pages.push(page);
                self.next = next;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }
}

pub fn use_infinite_pages<K, T, F, Fut>(
    key: K,
    fetch: F,
) -> (State<InfinitePages<T>>, EventHandler<()>)
where
    K: Clone + PartialEq + 'static,
    T: Clone + 'static,
    F: Fn(K, i64) -> Fut + Clone + 'static,
    Fut: Future<Output = Result<(T, Option<i64>), String>> + 'static,
{
    let source = use_reactive(&key);
    let mut data = use_state(InfinitePages::<T>::default);
    let mut task = use_hook(move || {
        FutureTask::create(move || {
            let key = source.peek().clone();
            let cursor = data.peek().next.unwrap_or(0);
            let fetch = fetch.clone();
            data.write().loading = true;
            data.write().error = None;
            async move {
                let result = fetch(key.clone(), cursor).await;
                if *source.peek() == key {
                    data.write().complete(result);
                }
            }
        })
    });
    use_side_effect_with_deps(&key, move |_| {
        task.cancel();
        data.set(InfinitePages {
            loading: true,
            ..Default::default()
        });
        task.start();
    });
    let more = EventHandler::new(move |()| {
        let ready = {
            let state = data.peek();
            !state.loading && state.has_more()
        };
        if ready {
            data.write().loading = true;
            task.start();
        }
    });
    (data, more)
}
