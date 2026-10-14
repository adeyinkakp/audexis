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
    let mut bounds = use_state(|| None::<Bounds>);
    let viewports = use_try_consume::<Viewports>().unwrap_or_default();
    let mut intersection = *bounds.read();
    for viewport in viewports.0 {
        let viewport = *viewport.read();
        intersection = intersection
            .zip(viewport)
            .and_then(|(a, b)| a.intersection(b));
    }
    (intersection.is_some(), move |event| {
        bounds.set_if_modified(Some(Bounds::from_event(&event)));
    })
}
