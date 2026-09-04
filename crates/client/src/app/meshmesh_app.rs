#[cfg(feature = "gui")]
use dioxus_native::prelude::*;

#[cfg(feature = "gui")]
#[component]
pub fn MeshmeshApp() -> Element {
    rsx! {
        div {
            h1 { "welcome to my app" }
        }
    }
}