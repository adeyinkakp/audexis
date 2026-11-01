mod api;
mod fields;
mod review;

use super::*;
pub use api::import_image;
use fields::{availability, fields, subtitle, title};
use serde_json::Value;

#[derive(PartialEq)]
pub struct Lookup {
    pub kind: Tab,
}
impl Component for Lookup {
    fn render(&self) -> impl IntoElement {
        let session = use_consume::<State<Session>>();
        let runtime = use_consume::<Handle>();
        let colors = crate::ui::theme::use_audexis_colors();
        let kind = self.kind;
        let mut open = use_state(|| false);
        let query = use_state(|| {
            format!(
                "{} {}",
                session.peek().text(FrameKey::Title),
                session.peek().text(FrameKey::Artist)
            )
            .trim()
            .to_owned()
        });
        let mut results = use_state(Vec::<Value>::new);
        let mut selected = use_state(|| None::<usize>);
        let mut chosen = use_state(Vec::<FrameKey>::new);
        let mut searching = use_state(|| false);
        let mut searched = use_state(|| false);
        let mut error = use_state(String::new);
        let busy = session.read().busy;
        let run_search = EventHandler::new(move |_: ()| {
            let term = query.peek().trim().to_owned();
            if term.is_empty() || *searching.peek() || session.peek().busy {
                return;
            }
            let runtime = runtime.clone();
            searching.set(true);
            searched.set(true);
            selected.set(None);
            chosen.set(Vec::new());
            results.set(Vec::new());
            error.set(String::new());
            spawn(async move {
                match runtime.spawn(api::search(kind, term)).await {
                    Ok(Ok(matches)) => results.set(matches),
                    Ok(Err(message)) => error.set(message),
                    Err(_) => error.set("Search could not finish. Please try again.".into()),
                }
                searching.set(false);
            });
        });
        let heading = if kind == Tab::Details {
            "Find metadata online"
        } else {
            "Find lyrics online"
        };
        if !*open.read() {
            return Button::new()
                .outline()
                .enabled(!busy)
                .child(heading)
                .on_press(move |_| open.set(true))
                .into_element();
        }
        let submit = run_search.clone();
        let mut body = rect().width(Size::fill()).padding(20.).spacing(16.)
            .corner_radius(16.).background(colors.surface_subtle)
            .border(Border::new().fill(colors.border).width(1.))
            .child(rect().horizontal().width(Size::fill()).content(Content::Flex)
                .cross_align(Alignment::Center)
                .child(rect().width(Size::flex(1.)).spacing(4.)
                    .child(label().text(heading).font_size(18.))
                    .child(label().text(if kind == Tab::Details {
                        "Apple Music · Choose a match, then select the fields to use."
                    } else {
                        "LRCLIB · Choose a match and review the lyrics before applying."
                    }).font_size(12.).color(colors.muted_foreground)))
                .child(Button::new().flat().compact().child("Close").on_press(move |_| open.set(false))))
            .child(rect().horizontal().width(Size::fill()).content(Content::Flex).spacing(10.)
                .child(Input::new(query).enabled(!busy && !*searching.read())
                    .width(Size::flex(1.)).placeholder("Search by song title and artist")
                    .on_submit(move |_: String| submit.call(())))
                .child(Button::new().filled().enabled(!busy && !*searching.read())
                    .child(if *searching.read() { "Searching…" } else { "Search" })
                    .on_press(move |_| run_search.call(()))));
        if !error.read().is_empty() {
            body = body.child(
                label()
                    .text(error.read().clone())
                    .color(colors.destructive)
                    .font_size(12.),
            );
        }
        if results.read().is_empty() {
            return body
                .child(
                    rect()
                        .width(Size::fill())
                        .padding(24.)
                        .spacing(6.)
                        .child(
                            label()
                                .text(if *searching.read() {
                                    "Looking for matches…"
                                } else if !error.read().is_empty() {
                                    "Try searching again."
                                } else if *searched.read() {
                                    "No matches found"
                                } else {
                                    "Find the right match for your music"
                                })
                                .font_size(14.),
                        )
                        .child(
                            label()
                                .text(if *searched.read() {
                                    "Try a shorter title or a different artist spelling."
                                } else {
                                    "Search above to see results here."
                                })
                                .font_size(12.)
                                .color(colors.muted_foreground),
                        ),
                )
                .into_element();
        }
        let current = *selected.read();
        let matches = results.read().clone();
        let mut list = rect().width(Size::flex(1.)).spacing(8.).child(
            label()
                .text(format!("{} matches", matches.len()))
                .font_size(12.)
                .color(colors.muted_foreground),
        );
        list = list.child(
            ScrollView::new()
                .width(Size::fill())
                .height(Size::px(350.))
                .spacing(6.)
                .children(matches.iter().enumerate().map(|(index, item)| {
                    let keys: Vec<_> = fields(kind, item).iter().map(|field| field.key).collect();
                    Button::new()
                        .flat()
                        .width(Size::fill())
                        .enabled(!busy)
                        .theme_colors(ButtonColorsThemePartial {
                            background: Some(
                                (if current == Some(index) {
                                    colors.primary_washed
                                } else {
                                    colors.background
                                })
                                .into(),
                            ),
                            ..Default::default()
                        })
                        .child(
                            rect()
                                .width(Size::fill())
                                .spacing(5.)
                                .padding(6.)
                                .child(label().text(title(item)).max_lines(2).font_size(14.))
                                .child(
                                    label()
                                        .text(subtitle(kind, item))
                                        .max_lines(2)
                                        .font_size(12.)
                                        .color(colors.muted_foreground),
                                )
                                .child(
                                    label()
                                        .text(availability(kind, item))
                                        .max_lines(2)
                                        .font_size(11.)
                                        .color(colors.muted_foreground),
                                ),
                        )
                        .on_press(move |_| {
                            if *selected.peek() != Some(index) {
                                let first_selection = selected.peek().is_none();
                                selected.set(Some(index));
                                if first_selection || kind != Tab::Details {
                                    chosen.set(keys.clone());
                                }
                            }
                        })
                        .into_element()
                })),
        );
        let preview = current.and_then(|index| matches.get(index).cloned());
        body.child(
            rect()
                .horizontal()
                .width(Size::fill())
                .content(Content::Flex)
                .spacing(16.)
                .child(list)
                .child(rect().width(Size::flex(1.)).child(review::Review {
                    kind,
                    item: preview,
                    chosen,
                })),
        )
        .into_element()
    }
    fn render_key(&self) -> DiffKey {
        (&self.kind.title()).into()
    }
}
