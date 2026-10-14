use super::pages::{home::Home, songs::Songs};
use super::shell::Shell;
use freya::{prelude::*, router::*};
#[derive(Routable, Clone, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Shell)]
        #[route("/")]
        Home,
        #[route("/songs")]
        Songs,
}
