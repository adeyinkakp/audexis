use super::*;

pub(super) struct Fetcher {
    source: Arc<StreamRegistration>,
    client: reqwest::blocking::Client,
    url: Option<(String, Instant)>,
    pub(super) position: u64,
    pub(super) start: u64,
    pub(super) buffer: Vec<u8>,
    cache: Option<Cache>,
    next_size: u64,
    stopping: Arc<AtomicBool>,
}
impl Fetcher {
    pub(super) fn new(
        source: Arc<StreamRegistration>,
        stopping: Arc<AtomicBool>,
    ) -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| "Could not initialize audio streaming")?;
        let cache = Cache::new(&source.options);
        Ok(Self {
            source,
            client,
            url: None,
            position: 0,
            start: 0,
            buffer: Vec::new(),
            cache,
            next_size: PAGE,
            stopping,
        })
    }
    pub(super) fn fill(&mut self) -> io::Result<()> {
        let start = self.position / PAGE * PAGE;
        if let Some(cache) = &self.cache {
            if let Some(buffer) = cache.read(start, (self.source.size - start).min(PAGE) as usize) {
                self.start = start;
                self.buffer = buffer;
                return Ok(());
            }
        }
        self.position = start;
        if let Some(message) = limit_message(&self.source.options) {
            return Err(io::Error::other(message));
        }
        let issued = Instant::now();
        let end = self
            .position
            .saturating_add(self.next_size - 1)
            .min(self.source.size - 1);
        for attempt in 0..2 {
            if self.stopping.load(Ordering::Relaxed) {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "Playback stopped",
                ));
            }
            if self
                .url
                .as_ref()
                .is_none_or(|(_, issued)| issued.elapsed() > Duration::from_secs(300))
            {
                let url = (self.source.resolve)().map_err(|error| {
                    if cap_body(&error) {
                        record_limit(&self.source.options);
                    }
                    io::Error::other(error)
                })?;
                let parsed = reqwest::Url::parse(&url)
                    .map_err(|_| io::Error::other("Invalid stream URL"))?;
                if !matches!(parsed.scheme(), "http" | "https")
                    || !parsed.username().is_empty()
                    || parsed.password().is_some()
                {
                    return Err(io::Error::other("Invalid stream URL"));
                }
                self.url = Some((url, Instant::now()));
            }
            if self.stopping.load(Ordering::Relaxed) {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "Playback stopped",
                ));
            }
            let response = self
                .client
                .get(&self.url.as_ref().unwrap().0)
                .header("range", format!("bytes={}-{}", self.position, end))
                .header("accept-encoding", "identity")
                .send();
            let mut response = match response {
                Ok(response) => response,
                Err(_) if attempt == 0 => continue,
                Err(_) => return Err(io::Error::other("Cloud audio connection interrupted")),
            };
            let status = response.status();
            if attempt == 0
                && (status.as_u16() == 401 || status.as_u16() == 403 || status.is_server_error())
            {
                let mut body = String::new();
                let _ = (&mut response).take(8192).read_to_string(&mut body);
                if cap_body(&body) {
                    record_limit(&self.source.options);
                    return Err(io::Error::other(
                        "Storage daily limit reached. Cloud playback will retry after a cooldown",
                    ));
                }
                self.url = None;
                continue;
            }
            if !status.is_success() {
                let request_id = response
                    .headers()
                    .get("x-amz-request-id")
                    .and_then(|v| v.to_str().ok())
                    .filter(|v| {
                        v.len() <= 128
                            && v.bytes()
                                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
                    })
                    .map(str::to_owned);
                let mut body = String::new();
                let _ = (&mut response).take(8192).read_to_string(&mut body);
                let code = body
                    .split("<Code>")
                    .nth(1)
                    .and_then(|s| s.split("</Code>").next())
                    .filter(|s| {
                        !s.is_empty()
                            && s.len() <= 100
                            && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    });
                return Err(io::Error::other(format!(
                    "Cloud audio stream failed (HTTP {}){}{}",
                    status.as_u16(),
                    code.map(|c| format!(": {c}")).unwrap_or_default(),
                    request_id
                        .map(|id| format!("; request ID {id}"))
                        .unwrap_or_default(),
                )));
            }
            let expected = end - self.position + 1;
            let valid_range = status == reqwest::StatusCode::PARTIAL_CONTENT
                && response
                    .headers()
                    .get("content-range")
                    .and_then(|v| v.to_str().ok())
                    == Some(
                        format!("bytes {}-{}/{}", self.position, end, self.source.size).as_str(),
                    );
            let entire_small_file = status == reqwest::StatusCode::OK
                && self.position == 0
                && expected == self.source.size;
            if !valid_range && !entire_small_file {
                return Err(io::Error::other("invalid audio byte range"));
            }
            let mut buffer = Vec::with_capacity(expected as usize);
            let read = (|| -> io::Result<()> {
                let mut input = (&mut response).take(expected + 1);
                let mut bytes = [0u8; 65536];
                loop {
                    if self.stopping.load(Ordering::Relaxed) {
                        return Err(io::Error::new(
                            io::ErrorKind::Interrupted,
                            "Playback stopped",
                        ));
                    }
                    let n = input.read(&mut bytes)?;
                    if n == 0 {
                        break;
                    }
                    buffer.extend_from_slice(&bytes[..n]);
                }
                Ok(())
            })();
            if read.is_err() || buffer.len() as u64 != expected {
                if attempt == 0 {
                    continue;
                }
                return Err(io::Error::other("Cloud audio range download interrupted"));
            }
            self.start = self.position;
            if let Some(cache) = &self.cache {
                for (index, page) in buffer.chunks(PAGE as usize).enumerate() {
                    cache.write(self.start + index as u64 * PAGE, page);
                }
            }
            self.buffer = buffer;
            let rate = expected as f64 / issued.elapsed().as_secs_f64().max(0.001);
            self.next_size = ((rate * 0.25) as u64 / PAGE * PAGE).clamp(PAGE, MAX_WINDOW);
            return Ok(());
        }
        unreachable!()
    }
}
