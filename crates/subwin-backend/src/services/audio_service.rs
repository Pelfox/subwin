use subwin_bridge::audio::InputDevice;

/// Handles an incoming audio devices list request (see
/// [`subwin_bridge::MessageToBackend::AudioDevicesListRequest`]).
pub async fn handle_audio_devices_list_request(context: super::AppContextHandle) {
    let (config, host) = {
        let state = context.state.read().await;
        (state.config.clone(), state.active_host.clone())
    };

    let devices = subwin_audio::device::list_host_input_devices(&host)
        .expect("failed to obtain host's input devices");
    let response_devices: Vec<InputDevice> = devices
        .iter()
        .map(|device| InputDevice {
            id: device.id.to_string(),
            description: device.description.clone(),
            selected: config.audio_device_config.selected_device_id == Some(device.id.to_string()),
        })
        .collect();

    context
        .send(subwin_bridge::MessageFromBackend::AudioDevicesListResponse(
            response_devices,
        ))
        .await;
}

/// Handles an audio device selection request and persists it to config.
pub async fn handle_audio_device_selection(context: super::AppContextHandle, id: String) {
    let active_host = {
        let state = context.state.read().await;
        state.active_host.clone()
    };

    let audio_device = subwin_audio::device::get_device_by_id(&active_host, id.clone())
        .expect("failed to get target device id");

    match audio_device {
        Some(device) => {
            let mut state = context.state.write().await;
            state.active_audio_device = std::sync::Arc::new(Some(device));
            state.config.audio_device_config.selected_device_id = Some(id);
            // persist the updated selection so it is remembered across runs
            crate::config::save_config(&state.config)
                .await
                .expect("failed to update selected device id");
        }
        None => log::error!("Could not find the target device at {}", id),
    }
}
