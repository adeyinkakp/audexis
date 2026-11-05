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
}

pub fn use_infinite_pages<K, T, F, Fut>(
    key: K,
    revision: u64,
    fetch: F,
) -> (State<InfinitePages<T>>, EventHandler<()>)
where
    K: Clone + PartialEq + 'static,
    T: Clone + PartialEq + 'static,
    F: Fn(K, i64) -> Fut + Clone + 'static,
    Fut: Future<Output = Result<(T, Option<i64>), String>> + 'static,
{
    let source = use_reactive(&(key.clone(), revision));
    let mut data = use_state(InfinitePages::<T>::default);
    let mut refreshing = use_state(|| false);
    let mut previous = use_state(|| None::<K>);
    let mut task = use_hook(move || {
        FutureTask::create(move || {
            let request = source.peek().clone();
            let replace = *refreshing.peek();
            let count = if replace {
                data.peek().pages.len().max(1)
            } else {
                1
            };
            let cursor = if replace {
                0
            } else {
                data.peek().next.unwrap_or(0)
            };
            let fetch = fetch.clone();
            async move {
                let result = fetch_pages(request.0.clone(), cursor, count, fetch).await;
                if *source.peek() == request {
                    refreshing.set(false);
                    if replace {
                        if let Ok((pages, next)) = &result {
                            let current = data.peek();
                            if &current.pages == pages
                                && &current.next == next
                                && current.error.is_none()
                                && !current.loading
                            {
                                return;
                            }
                        }
                    }
                    let mut state = data.write();
                    state.loading = false;
                    match result {
                        Ok((pages, next)) => {
                            if replace {
                                state.pages = pages;
                            } else {
                                state.pages.extend(pages);
                            }
                            state.next = next;
                            state.error = None;
                        }
                        Err(error) => state.error = Some(error),
                    }
                }
            }
        })
    });
    use_side_effect_with_deps(&(key, revision), move |(key, _)| {
        task.cancel();
        if previous.peek().as_ref() != Some(key) {
            previous.set(Some(key.clone()));
            refreshing.set(false);
            data.set(InfinitePages {
                loading: true,
                ..Default::default()
            });
        } else {
            refreshing.set(true);
        }
        task.start();
    });
    let more = EventHandler::new(move |()| {
        let ready = !*refreshing.peek() && {
            let state = data.peek();
            !state.loading && state.has_more()
        };
        if ready {
            data.write().loading = true;
            data.write().error = None;
            task.start();
        }
    });
    (data, more)
}

async fn fetch_pages<K, T, F, Fut>(
    key: K,
    mut cursor: i64,
    count: usize,
    fetch: F,
) -> Result<(Vec<T>, Option<i64>), String>
where
    K: Clone,
    F: Fn(K, i64) -> Fut,
    Fut: Future<Output = Result<(T, Option<i64>), String>>,
{
    let mut pages = Vec::new();
    let mut next = None;
    for _ in 0..count {
        let (page, following) = fetch(key.clone(), cursor).await?;
        pages.push(page);
        next = following;
        match following {
            Some(value) if value > cursor => cursor = value,
            Some(_) => return Err("Library returned a non-advancing page".into()),
            None => break,
        }
    }
    Ok((pages, next))
}
