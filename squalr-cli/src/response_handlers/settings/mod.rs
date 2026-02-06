use squalr_engine_api::commands::settings::settings_response::SettingsResponse;

pub fn handle_settings_response(cmd: SettingsResponse) {
    // Settings responses are primarily consumed by the GUI. For CLI usage, just log them.
    log::info!("{:?}", cmd);
}
