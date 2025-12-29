pub mod audio;
pub mod math;
pub mod speech;

use cpal::traits::StreamTrait;
use log::{error, info};
use ringbuf_blocking::traits::{Consumer, Producer, Split};
use std::{io::Write, sync::mpsc, thread};
use whisper_rs::{FullParams, WhisperContextParameters};

use crate::{audio::resampler::AudioResampler, speech::Transcoder};

const TARGET_SAMPLE_RATE: u32 = 16_000; // 16 kHz

fn prompt_select_capture_device(host: &cpal::Host) -> audio::device::HostInputDevice {
    let devices =
        audio::device::list_host_input_devices(host).expect("failed to list host input devices");
    for (index, device) in devices.iter().enumerate() {
        println!("[SELECT] {}. Input device: {device}", index + 1);
    }

    print!("[INFO] Select the capture device to use: ");
    std::io::stdout().flush().unwrap();

    let mut capture_device_index = String::new();
    if let Err(e) = std::io::stdin().read_line(&mut capture_device_index) {
        panic!("failed to read line: {}", e);
    }

    let capture_device_index = capture_device_index
        .trim()
        .parse::<usize>()
        .expect("invalid input")
        - 1;

    match devices.get(capture_device_index) {
        Some(device) => device.clone(),
        None => panic!("no device found at index {}", capture_device_index + 1),
    }
}

fn main() {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .without_timestamps()
        .init()
        .expect("failed to create logger instance");
    whisper_rs::install_logging_hooks();

    let capture_device = prompt_select_capture_device(&cpal::default_host());
    println!("[INFO] Using capture device: {capture_device}");

    let (sample_rate, channels) = capture_device
        .sample_rate_and_channels()
        .expect("failed to get device's sample_rate and channels");
    let target_buffer_size = capture_device
        .target_buffer_size(TARGET_SAMPLE_RATE)
        .expect("failed to get device's target buffer size");

    println!(
        "[INFO] Device encodes data at {} Hz, has {} channel(-s) and selected target buffer size is {}",
        sample_rate, channels, target_buffer_size
    );

    // TODO: support for changing the resampler
    let mut resampler = audio::resampler::FixedBlockResampler::<f32>::new(
        sample_rate,
        TARGET_SAMPLE_RATE,
        target_buffer_size,
    )
    .expect("failed to create FixedBlockResampler");

    let ring_buffer =
        ringbuf_blocking::BlockingHeapRb::<f32>::new((TARGET_SAMPLE_RATE * 3) as usize);
    let (mut producer, mut consumer) = ring_buffer.split();

    thread::spawn(move || {
        let mut context_params = WhisperContextParameters::default();
        context_params.use_gpu(true);
        let mut transcoder = crate::speech::whisper::WhisperTranscoder::new(
            TARGET_SAMPLE_RATE,
            "./ggml-small-q5_1.bin",
            context_params,
        )
        .expect("failed to create transcoder");

        let mut params = FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_special(false);
        params.set_print_timestamps(false);
        params.set_debug_mode(false);
        params.set_language(Some("en"));
        params.set_n_threads(num_cpus::get_physical() as i32);

        params.set_no_timestamps(false);
        params.set_token_timestamps(false);
        params.set_single_segment(true);
        params.set_max_tokens(96);

        let mut samples_buffer = vec![0.0f32; target_buffer_size as usize];
        loop {
            let len = consumer.pop_slice(&mut samples_buffer);
            if len == 0 {
                continue;
            }

            transcoder.accept_samples(&samples_buffer[..len]);
            if let Some(value) = transcoder.try_transcode(params.clone()) {
                // print!("\r\x1b[2K{value}");
                // std::io::stdout().flush().unwrap();
                println!("{value}");
            };

            // Finalize chunk (optional)
            // if segment_window.len() >= length_samples {
            //     if keep_samples > 0 && segment_window.len() > keep_samples {
            //         segment_window.drain(..segment_window.len() - keep_samples);
            //     } else {
            //         segment_window.clear();
            //     }
            // }
        }
    });

    let mut samples_accumulator = Vec::new();
    let mut resampled_callback = move |written_data: &[f32]| {
        producer.push_slice(written_data);
    };

    let handle_samples_data = move |samples_frame_data: &[f32]| {
        if samples_frame_data.len() != (target_buffer_size as usize * channels as usize) {
            error!(
                "CPAL delivered unexpected buffer of {} samples",
                samples_frame_data.len()
            );
            // TODO: switch to StreamingResampler here
        }
        let frames = samples_frame_data.len() / channels as usize;
        samples_accumulator.resize(frames, 0.0);
        audio::mixer::mix_stereo_to_mono(&mut samples_accumulator[..frames], samples_frame_data);

        match resampler.process_callback(&samples_accumulator[..frames], &mut resampled_callback) {
            Ok(_) => {}
            Err(e) => error!(
                "resampler error: {e:?}, frames={}, buffer_size={}",
                frames, target_buffer_size
            ),
        }
    };

    let handle_error = move |error| error!("Error: {error}");
    let capture_device_stream = audio::device::open_cpal_input_stream(
        &capture_device,
        TARGET_SAMPLE_RATE,
        handle_samples_data,
        handle_error,
    )
    .expect("failed to create stream");

    capture_device_stream
        .play()
        .expect("failed to start the stream");
    println!("[INFO] Capturing audio... press Ctrl+C to stop");

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
