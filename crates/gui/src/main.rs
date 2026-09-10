mod app;

use app::MeshmeshApp;

fn main() {
    tracing_subscriber::fmt::init();
    dioxus_native::launch(MeshmeshApp);
}
