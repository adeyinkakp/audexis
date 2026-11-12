use super::*;

pub(super) fn encode(s: &str, slash: bool) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) || (slash && b == b'/') {
            out.push(b as char)
        } else {
            out.push_str(&format!("%{b:02X}"))
        }
    }
    out
}
pub(super) fn mac(key: &[u8], data: &str) -> Vec<u8> {
    let mut m = Hmac::<Sha256>::new_from_slice(key).unwrap();
    m.update(data.as_bytes());
    m.finalize().into_bytes().to_vec()
}
pub(super) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub(super) fn sign(
    endpoint: &str,
    region: &str,
    bucket: &str,
    key_id: &str,
    secret: &str,
    method: &str,
    key: &str,
    length: Option<i64>,
    date: &str,
    expiry: u32,
    extra: &[(&str, String)],
) -> Result<String> {
    let origin =
        reqwest::Url::parse(endpoint).map_err(|_| Error::bad("Invalid storage endpoint"))?;
    let host = origin
        .host_str()
        .ok_or_else(|| Error::bad("Invalid storage endpoint"))?;
    let host = if let Some(port) = origin.port() {
        format!("{host}:{port}")
    } else {
        host.to_owned()
    };
    let path = format!(
        "{}/{}/{}",
        origin.path().trim_end_matches('/'),
        encode(bucket, false),
        encode(key, true)
    );
    let scope = format!("{}/{region}/s3/aws4_request", &date[..8]);
    let signed_headers = if length.is_some() {
        "content-length;host"
    } else {
        "host"
    };
    let mut query = BTreeMap::from([
        ("X-Amz-Algorithm", "AWS4-HMAC-SHA256".to_owned()),
        ("X-Amz-Credential", format!("{key_id}/{scope}")),
        ("X-Amz-Date", date.to_owned()),
        ("X-Amz-Expires", expiry.to_string()),
        ("X-Amz-SignedHeaders", signed_headers.to_owned()),
    ]);
    for (key, value) in extra {
        query.insert(key, value.clone());
    }
    let canonical_query = query
        .iter()
        .map(|(k, v)| format!("{}={}", encode(k, false), encode(v, false)))
        .collect::<Vec<_>>()
        .join("&");
    let headers = match length {
        Some(n) => format!("content-length:{n}\nhost:{host}\n"),
        None => format!("host:{host}\n"),
    };
    let canonical = format!(
        "{method}\n{path}\n{canonical_query}\n{headers}\n{signed_headers}\nUNSIGNED-PAYLOAD"
    );
    let to_sign = format!(
        "AWS4-HMAC-SHA256\n{date}\n{scope}\n{:x}",
        Sha256::digest(canonical.as_bytes())
    );
    let day = mac(format!("AWS4{secret}").as_bytes(), &date[..8]);
    let reg = mac(&day, region);
    let service = mac(&reg, "s3");
    let signing = mac(&service, "aws4_request");
    query.insert("X-Amz-Signature", hex(&mac(&signing, &to_sign)));
    Ok(format!(
        "{}://{host}{path}?{}",
        origin.scheme(),
        query
            .iter()
            .map(|(k, v)| format!("{}={}", encode(k, false), encode(v, false)))
            .collect::<Vec<_>>()
            .join("&")
    ))
}
