/// Handles an incoming configuration request (see
/// [`subwin_bridge::MessageToBackend::ConfigurationRequest`]).
pub async fn handle_config_request(context: super::AppContextHandle) {
    let config = {
        let state = context.state.read().await;
        state.config.clone()
    };
    context
        .send(subwin_bridge::MessageFromBackend::ConfigurationResponse(
            config,
        ))
        .await;
}
