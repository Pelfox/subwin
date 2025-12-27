fn main() {
    simple_logger::SimpleLogger::new()
        .with_colors(true)
        .with_threads(true)
        .with_local_timestamps()
        .init()
        .expect("failed to build logger instance");

    let channels = subwin_bridge::BridgeChannels::default();
    subwin_backend::run(channels.backend_rx, channels.backend_tx);
    subwin_frontend::run(channels.frontend_rx, channels.frontend_tx)
        .expect("failed to run frontend");
}
