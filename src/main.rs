fn main() {
    let channels = subwin_bridge::BridgeChannels::default();

    subwin_backend::run(channels.backend_rx, channels.backend_tx);
    subwin_frontend::run(channels.frontend_rx, channels.frontend_tx)
        .expect("failed to run frontend");
}
