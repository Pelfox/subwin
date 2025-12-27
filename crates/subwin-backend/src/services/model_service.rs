use std::str::FromStr;

use futures_util::StreamExt;
use reqwest::Url;
use subwin_bridge::whisper_model::WhisperModel;
use tokio::io::AsyncWriteExt;

const BASE_DOWNLOAD_PATH: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/";

/// Builds the download URL for the given Whisper model.
///
/// This function maps a [`WhisperModel`] variant to its corresponding model
/// file name and constructs the full download URL using the predefined base
/// download path.
///
/// # Returns
/// - The model file name.
/// - The full URL from which the model can be downloaded.
fn build_download_url(model: &WhisperModel) -> (&str, Url) {
    let model_file_name = match model {
        WhisperModel::TinyQuantized8 => "ggml-tiny-q8_0.bin",
        WhisperModel::TinyQuantized5 => "ggml-tiny-q5_1.bin",
        WhisperModel::Tiny => "ggml-tiny.bin",
        WhisperModel::SmallQuantized8 => "ggml-small-q8_0.bin",
        WhisperModel::SmallQuantized5 => "ggml-small-q5_1.bin",
        WhisperModel::Small => "ggml-small.bin",
        WhisperModel::BaseQuantized8 => "ggml-base-q8_0.bin",
        WhisperModel::BaseQuantized5 => "ggml-base-q5_1.bin",
        WhisperModel::Base => "ggml-base.bin",
        WhisperModel::MediumQuantized8 => "ggml-medium-q8_0.bin",
        WhisperModel::MediumQuantized5 => "ggml-medium-q5_0.bin",
        WhisperModel::Medium => "ggml-medium.bin",
        WhisperModel::LargeTurboQuantized8 => "ggml-large-v3-turbo-q8_0.bin",
        WhisperModel::LargeTurboQuantized5 => "ggml-large-v3-turbo-q5_0.bin",
        WhisperModel::LargeTurbo => "ggml-large-v3-turbo.bin",
        WhisperModel::LargeQuantized5 => "ggml-large-v3-q5_0.bin",
        WhisperModel::Large => "ggml-large-v3.bin",
    };

    let model_url = Url::from_str(BASE_DOWNLOAD_PATH)
        .expect("failed to build a base HuggingFace URL")
        .join(model_file_name)
        .expect("failed to append model's file name");

    (model_file_name, model_url)
}

/// Handles an incoming model download request (see
/// [`subwin_bridge::MessageToBackend::DownloadModelRequest`]).
pub async fn handle_download_model_request(
    context: &crate::AppContext,
    model: subwin_bridge::whisper_model::WhisperModel,
) {
    let (mut config, request_client, cache_path) = {
        let state = context.state.read().await;
        (
            state.config.clone(),
            state.request_client.clone(),
            state.cache_path.clone(),
        )
    };

    let (model_file_name, model_download_url) = build_download_url(&model);
    let save_path = cache_path.join(model_file_name);
    log::info!("Downloading model {model:?} from {model_download_url}, saving to {save_path:?}");

    if let Some(parent) = save_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .expect("failed to create cache directory");
    }

    let mut output_file = tokio::fs::File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open(save_path.clone())
        .await
        .expect("failed to create model file");

    let request = request_client
        .get(model_download_url)
        .build()
        .expect("failed to build model download request");

    let context = context.clone();
    tokio::spawn(async move {
        match request_client.execute(request).await {
            Ok(response) => {
                let start = tokio::time::Instant::now();
                let total_bytes = response.content_length().unwrap_or(0);
                let mut downloaded_bytes = 0u64;

                let mut body = response.bytes_stream();
                while let Some(chunk) = body.next().await {
                    let current_chunk = chunk.expect("failed to get current file chunk");
                    output_file
                        .write_all(&current_chunk)
                        .await
                        .expect("failed to write current file chunk");
                    downloaded_bytes += current_chunk.len() as u64;

                    let elapsed_secs = start.elapsed().as_secs_f64();
                    let speed = downloaded_bytes as f64 / elapsed_secs;
                    let remaining_time = (total_bytes - downloaded_bytes) as f64 / speed;

                    // notify frontend about current state
                    context
                        .send(subwin_bridge::MessageFromBackend::DownloadProgressUpdate {
                            speed,
                            downloaded_bytes,
                            total_bytes,
                            remaining_time,
                        })
                        .await;
                }

                // update config with new path
                config.active_model_path = Some(save_path);
                crate::config::save_config(&config)
                    .await
                    .expect("failed to update active model path");
            }
            Err(e) => {
                context
                    .send_notification(
                        subwin_bridge::notification::NotificationType::Error,
                        e.without_url().to_string(),
                    )
                    .await
            }
        }
    });
}
