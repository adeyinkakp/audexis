use super::*;
use serde_json::Value;
use std::time::Duration;

pub fn import_image(data: Vec<u8>) -> Result<TagValue, String> {
    if data.is_empty() || data.len() > 20 * 1024 * 1024 {
        return Err("Artwork must be between 1 byte and 20 MB.".into());
    }
    let format = image::guess_format(&data).map_err(|e| e.to_string())?;
    let mime = match format {
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::Png => "image/png",
        _ => return Err("Use a JPEG or PNG image.".into()),
    };
    let mut reader = image::ImageReader::new(std::io::Cursor::new(&data));
    reader.set_format(format);
    let (width, height) = reader.into_dimensions().map_err(|e| e.to_string())?;
    if u64::from(width) * u64::from(height) > 40_000_000 {
        return Err("Artwork is too large (maximum 40 megapixels).".into());
    }
    Ok(TagValue::Picture {
        mime: mime.into(),
        data,
        picture_type: Some(3),
        description: Some("Front cover".into()),
    })
}
async fn bytes(url: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("Audexis/0.20.0")
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let mut data = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if data.len() + chunk.len() > 20 * 1024 * 1024 {
            return Err("Response exceeds 20 MB.".into());
        }
        data.extend_from_slice(&chunk);
    }
    Ok(data)
}
async fn search(kind: Tab, term: String) -> Result<Vec<Value>, String> {
    let mut url = reqwest::Url::parse(if kind == Tab::Details {
        "https://itunes.apple.com/search"
    } else {
        "https://lrclib.net/api/search"
    })
    .unwrap();
    if kind == Tab::Details {
        url.query_pairs_mut()
            .append_pair("term", &term)
            .append_pair("media", "music")
            .append_pair("entity", "song")
            .append_pair("limit", "50");
    } else {
        url.query_pairs_mut().append_pair("q", &term);
    }
    let data = bytes(url.as_str()).await?;
    let value: Value = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
    let values = if kind == Tab::Details {
        value.get("results")
    } else {
        Some(&value)
    }
    .and_then(Value::as_array)
    .ok_or("Invalid search response")?;
    Ok(values
        .iter()
        .filter(|v| kind != Tab::Details || v["kind"] == "song")
        .cloned()
        .collect())
}
#[derive(PartialEq)]
pub struct Lookup {
    pub kind: Tab,
}
impl Component for Lookup {
    fn render(&self) -> impl IntoElement {
        let mut session = use_consume::<State<Session>>();
        let runtime = use_consume::<Handle>();
        let colors = super::super::theme::use_audexis_colors();
        let kind = self.kind;
        let mut open = use_state(|| false);
        let query = use_state(|| {
            format!(
                "{} {}",
                session.peek().text(FrameKey::Title),
                session.peek().text(FrameKey::Artist)
            )
        });
        let mut results = use_state(Vec::<Value>::new);
        let mut selected = use_state(|| None::<usize>);
        let mut searching = use_state(|| false);
        let mut error = use_state(String::new);
        let busy = session.read().busy;
        let search_runtime = runtime.clone();
        let mut body = rect().width(Size::fill()).spacing(8.).child(
            Button::new()
                .enabled(!busy)
                .child(if *open.read() {
                    "Hide online lookup"
                } else if kind == Tab::Details {
                    "Find metadata online"
                } else {
                    "Find lyrics online"
                })
                .on_press(move |_| open.toggle()),
        );
        if !*open.read() {
            return body.into_element();
        }
        body = body
            .padding(16.)
            .corner_radius(12.)
            .background(colors.surface_subtle)
            .border(Border::new().fill(colors.border).width(1.))
            .child(
                label()
                    .text("Song and artist")
                    .font_size(12.)
                    .color(colors.muted_foreground),
            )
            .child(
                rect()
                    .horizontal()
                    .spacing(8.)
                    .content(Content::Flex)
                    .child(
                        Input::new(query)
                            .width(Size::flex(1.))
                            .placeholder("Song and artist"),
                    )
                    .child(
                        Button::new()
                            .enabled(!busy && !*searching.read())
                            .child("Search")
                            .on_press(move |_| {
                                let term = query.peek().trim().to_string();
                                if term.is_empty() {
                                    return;
                                }
                                let runtime = search_runtime.clone();
                                searching.set(true);
                                selected.set(None);
                                error.set(String::new());
                                spawn(async move {
                                    match runtime.spawn(search(kind, term)).await {
                                        Ok(Ok(matches)) => results.set(matches),
                                        other => error.set(format!("Search failed: {other:?}")),
                                    };
                                    searching.set(false);
                                });
                            }),
                    ),
            )
            .child(label().text(if *searching.read() {
                "Searching…".into()
            } else if results.read().is_empty() {
                "No results. Try a song title and artist.".into()
            } else {
                format!("{} results", results.read().len())
            }))
            .child(label().text(error.read().clone()));
        body = body.child(ScrollView::new().height(Size::px(140.)).children(
            results.read().iter().enumerate().map(|(index, item)| {
                Button::new()
                    .flat()
                    .child(format!(
                        "{} — {} · {}",
                        item["trackName"].as_str().unwrap_or("Unknown track"),
                        item["artistName"].as_str().unwrap_or(""),
                        item[if kind == Tab::Details {
                            "collectionName"
                        } else {
                            "albumName"
                        }]
                        .as_str()
                        .unwrap_or("")
                    ))
                    .on_press(move |_| selected.set(Some(index)))
                    .into_element()
            }),
        ));
        if let Some(item) = selected.read().and_then(|i| results.read().get(i).cloned()) {
            let lyric_key = if kind == Tab::Synced {
                "syncedLyrics"
            } else {
                "plainLyrics"
            };
            let preview = if kind == Tab::Details {
                format!(
                    "{}\n{}\n{}\nGenre: {} · Year: {}",
                    item["trackName"],
                    item["artistName"],
                    item["collectionName"],
                    item["primaryGenreName"],
                    item["releaseDate"]
                )
            } else {
                item[lyric_key]
                    .as_str()
                    .unwrap_or("No lyrics of this type available.")
                    .to_string()
            };
            body = body.child(label().text(preview)).child(
                Button::new()
                    .enabled(
                        !busy
                            && (kind == Tab::Details
                                || item[lyric_key].as_str().is_some_and(|s| !s.is_empty())),
                    )
                    .child("Use selected result")
                    .on_press(move |_| {
                        if kind != Tab::Details {
                            let text = item[lyric_key].as_str().unwrap_or_default().to_string();
                            if kind == Tab::Synced {
                                if let Err(e) = metadata_editor::parse_lrc(&text) {
                                    session.write().message = e;
                                    return;
                                }
                            }
                            session.write().set_values(
                                if kind == Tab::Synced {
                                    FrameKey::SynchronizedLyrics
                                } else {
                                    FrameKey::UnsyncedLyrics
                                },
                                vec![TagValue::Text(text)],
                            );
                            session.write().revision += 1;
                        } else {
                            let item = item.clone();
                            let runtime = runtime.clone();
                            session.write().busy = true;
                            spawn(async move {
                                let artwork_url =
                                    item["artworkUrl100"].as_str().map(str::to_string);
                                let artwork = if let Some(url) = artwork_url {
                                    runtime
                                        .spawn(async move {
                                            let parsed = reqwest::Url::parse(&url)
                                                .map_err(|e| e.to_string())?;
                                            if parsed.scheme() != "https"
                                                || !parsed
                                                    .host_str()
                                                    .is_some_and(|h| h.ends_with(".mzstatic.com"))
                                                || !parsed.username().is_empty()
                                                || parsed.password().is_some()
                                                || parsed.port().is_some()
                                            {
                                                return Err("Unsupported artwork URL".into());
                                            }
                                            import_image(
                                                bytes(&url.replace("100x100bb", "1000x1000bb"))
                                                    .await?,
                                            )
                                        })
                                        .await
                                        .map_err(|e| e.to_string())
                                        .and_then(|r| r)
                                        .map(Some)
                                } else {
                                    Ok(None)
                                };
                                let mut s = session.write();
                                for (field, source) in [
                                    (FrameKey::Title, "trackName"),
                                    (FrameKey::Artist, "artistName"),
                                    (FrameKey::Album, "collectionName"),
                                    (FrameKey::Genre, "primaryGenreName"),
                                    (FrameKey::Year, "releaseDate"),
                                    (FrameKey::TrackNumber, "trackNumber"),
                                    (FrameKey::DiscNumber, "discNumber"),
                                ] {
                                    let value = &item[source];
                                    let mut text = value
                                        .as_str()
                                        .map(str::to_string)
                                        .or_else(|| value.as_u64().map(|v| v.to_string()))
                                        .unwrap_or_default();
                                    if field == FrameKey::Year {
                                        text = text.chars().take(4).collect();
                                    }
                                    if !text.trim().is_empty() {
                                        s.set_values(field, vec![TagValue::Text(text)]);
                                    }
                                }
                                match artwork {
                                    Ok(Some(image)) => {
                                        for file in &mut s.files {
                                            let images = file
                                                .tags
                                                .entry(FrameKey::AttachedPicture)
                                                .or_default();
                                            images.retain(|v| {
                                                !matches!(
                                                    v,
                                                    TagValue::Picture {
                                                        picture_type: Some(3),
                                                        ..
                                                    }
                                                )
                                            });
                                            images.insert(0, image.clone());
                                        }
                                    }
                                    Ok(None) => {}
                                    Err(e) => {
                                        s.message = format!(
                                            "Metadata imported, but cover download failed: {e}"
                                        )
                                    }
                                }
                                s.busy = false;
                                s.revision += 1;
                            });
                        }
                    }),
            );
        }
        body.into_element()
    }
    fn render_key(&self) -> DiffKey {
        (&self.kind.title()).into()
    }
}
