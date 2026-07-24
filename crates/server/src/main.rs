fn main() {
    let _logging = spacegame2d_logging::init("spacegame2d-server", "info")
        .expect("failed to initialize logging");
    tracing::info!(
        event = "server_starting",
        address = "127.0.0.1:4000",
        "spacegame2d-server starting"
    );
}
