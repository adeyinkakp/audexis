use freya::prelude::*;

#[derive(Clone, Copy, PartialEq, Debug)]
struct Bounds {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}
impl Bounds {
    fn from_event(event: &SizedEventData) -> Self {
        let area = event.visible_area;
        Self {
            left: area.min_x(),
            top: area.min_y(),
            right: area.max_x(),
            bottom: area.max_y(),
        }
    }
    fn intersection(self, other: Self) -> Option<Self> {
        let result = Self {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.min(other.right),
            bottom: self.bottom.min(other.bottom),
        };
        (result.right > result.left && result.bottom > result.top).then_some(result)
    }
}
#[derive(Clone, Default)]
struct Viewports(Vec<State<Option<Bounds>>>);

#[derive(PartialEq, Default)]
pub struct VisibilityViewport {
    children: Vec<Element>,
}
impl VisibilityViewport {
    pub fn new() -> Self {
        Self::default()
    }
}
impl ChildrenExt for VisibilityViewport {
    fn get_children(&mut self) -> &mut Vec<Element> {
        &mut self.children
    }
}
impl Component for VisibilityViewport {
    fn render(&self) -> impl IntoElement {
        let mut bounds = use_state(|| None);
        let inherited = use_try_consume::<Viewports>().unwrap_or_default();
        use_provide_context(move || {
            let mut viewports = inherited;
            viewports.0.push(bounds);
            viewports
        });
        rect()
            .expanded()
            .on_sized(move |event: Event<SizedEventData>| {
                bounds.set_if_modified(Some(Bounds::from_event(&event)));
            })
            .children(self.children.clone())
    }
}

pub fn use_visibility() -> (bool, impl FnMut(Event<SizedEventData>) + 'static) {
    use_visibility_margin(0.)
}

pub(crate) fn use_visibility_margin(
    margin: f32,
) -> (bool, impl FnMut(Event<SizedEventData>) + 'static) {
    let mut bounds = use_state(|| None::<Bounds>);
    let viewports = use_try_consume::<Viewports>().unwrap_or_default();
    let mut intersection = *bounds.read();
    for viewport in viewports.0 {
        let viewport = (*viewport.read()).map(|bounds| Bounds {
            left: bounds.left - margin,
            top: bounds.top - margin,
            right: bounds.right + margin,
            bottom: bounds.bottom + margin,
        });
        intersection = intersection
            .zip(viewport)
            .and_then(|(a, b)| a.intersection(b));
    }
    (intersection.is_some(), move |event| {
        let area = if margin > 0. {
            event.area
        } else {
            event.visible_area
        };
        bounds.set_if_modified(Some(Bounds {
            left: area.min_x(),
            top: area.min_y(),
            right: area.max_x(),
            bottom: area.max_y(),
        }));
    })
}

pub(crate) fn use_visible_rows(
    count: usize,
    stride: f32,
) -> (
    std::ops::Range<usize>,
    impl FnMut(Event<SizedEventData>) + 'static,
) {
    let mut top = use_state(|| None::<f32>);
    let viewports = use_try_consume::<Viewports>().unwrap_or_default();
    let mut viewport: Option<Bounds> = None;
    for bounds in viewports.0 {
        let Some(bounds) = *bounds.read() else {
            viewport = None;
            break;
        };
        viewport = match viewport {
            Some(previous) => previous.intersection(bounds),
            None => Some(bounds),
        };
        if viewport.is_none() {
            break;
        }
    }
    let range = match (*top.read(), viewport) {
        (Some(top), Some(viewport)) => row_range(count, stride, top, viewport.top, viewport.bottom),
        _ => 0..0,
    };
    (range, move |event: Event<SizedEventData>| {
        top.set_if_modified(Some(event.area.min_y()));
    })
}

fn row_range(
    count: usize,
    stride: f32,
    top: f32,
    viewport_top: f32,
    viewport_bottom: f32,
) -> std::ops::Range<usize> {
    const OVERSCAN: f32 = 512.;
    let start = ((viewport_top - OVERSCAN - top).max(0.) / stride).floor() as usize;
    let end = ((viewport_bottom + OVERSCAN - top).max(0.) / stride).ceil() as usize;
    start.min(count)..end.min(count).max(start.min(count))
}
