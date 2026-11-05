use super::*;

#[derive(Debug, PartialEq)]
pub(super) enum Callback {
    Code(String),
    Error(String),
}

pub(super) fn parse_callback(request: &str, host: &str, state: &str) -> Result<Callback, ()> {
    let mut lines = request.split("\r\n");
    let mut parts = lines.next().ok_or(())?.split_whitespace();
    if parts.next() != Some("GET") {
        return Err(());
    }
    let target = parts.next().ok_or(())?;
    if parts.next() != Some("HTTP/1.1")
        || parts.next().is_some()
        || !target.starts_with("/callback?")
    {
        return Err(());
    }
    let hosts: Vec<_> = lines
        .filter_map(|line| line.split_once(':'))
        .filter(|(name, _)| name.eq_ignore_ascii_case("host"))
        .collect();
    if hosts.len() != 1 || hosts[0].1.trim() != host {
        return Err(());
    }
    let url = reqwest::Url::parse(&format!("http://{host}{target}")).map_err(|_| ())?;
    if url.path() != "/callback" || url.fragment().is_some() {
        return Err(());
    }
    let params: Vec<_> = url.query_pairs().collect();
    if params.len() != 2
        || params
            .iter()
            .filter(|(key, value)| key == "state" && value == state)
            .count()
            != 1
    {
        return Err(());
    }
    if let Some((_, code)) = params.iter().find(|(key, _)| key == "code") {
        if code.len() != 43
            || !code
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        {
            return Err(());
        }
        return Ok(Callback::Code(code.to_string()));
    }
    let error = params
        .iter()
        .find(|(key, _)| key == "error")
        .ok_or(())?
        .1
        .to_string();
    Ok(Callback::Error(error))
}

pub(super) async fn read_request(stream: &mut TcpStream) -> Result<String, String> {
    let mut request = Vec::new();
    loop {
        let mut chunk = [0u8; 1024];
        let count = stream
            .read(&mut chunk)
            .await
            .map_err(|_| "Could not read callback")?;
        if count == 0 {
            return Err("Incomplete callback".into());
        }
        request.extend_from_slice(&chunk[..count]);
        if request.len() > 8192 {
            return Err("Callback too large".into());
        }
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            return String::from_utf8(request).map_err(|_| "Invalid callback encoding".into());
        }
    }
}

pub(super) async fn browser_response(stream: &mut TcpStream, status: &str, message: &str) {
    let body = format!(
        "<!doctype html><html><head><meta charset='utf-8'></head><body <h1>{message}</h1></body></html>"
    );
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; frame-ancestors 'none'\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = tokio::time::timeout(Duration::from_secs(2), async {
        stream.write_all(response.as_bytes()).await?;
        stream.shutdown().await
    })
    .await;
}
