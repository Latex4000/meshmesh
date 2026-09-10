use dioxus_native::prelude::*;

static APP_CSS: Asset = asset!("../assets/style.css");

#[component]
pub fn MeshmeshApp() -> Element {
    rsx! {
        document::Stylesheet { href: APP_CSS }

        div {
            h1 { "welcome to my app" }
        }
    }
}
