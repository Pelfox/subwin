pub mod audio;
pub mod math;

use cpal::traits::StreamTrait;
use ringbuf_blocking::traits::{Consumer, Producer, Split};
use std::{io::Write, sync::mpsc, thread};

use crate::audio::resampler::AudioResampler;

const TARGET_SAMPLE_RATE: u32 = 16_000; // 16 kHz
const TARGET_RECORDING_DURATION: u32 = 3; // 3 seconds for testing

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
        Some(device) => device.clone(), // FIXME: try to remove this `clone`
        None => panic!("no device found at index {}", capture_device_index + 1),
    }
}

fn main() {
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

    let (worker_sender, worker_receiver) = mpsc::channel::<usize>();
    let ring_buffer =
        ringbuf_blocking::BlockingHeapRb::<f32>::new((target_buffer_size * 30) as usize);
    let (mut producer, mut consumer) = ring_buffer.split();

    thread::spawn(move || {
        let window_samples = TARGET_SAMPLE_RATE * TARGET_RECORDING_DURATION;
        let mut samples_accumulator = Vec::<f32>::with_capacity(window_samples as usize);

        loop {
            let written_length = worker_receiver
                .recv()
                .expect("failed to get written data as a receiver");
            let mut written_buffer = vec![0.0f32; written_length];

            consumer.pop_slice(&mut written_buffer);
            samples_accumulator.extend_from_slice(&written_buffer);

            // wait till we get the required amount of data to process
            if samples_accumulator.len() < window_samples as usize {
                continue;
            }

            // TODO: if data is enough, then start the transcription
            samples_accumulator.clear();
        }
    });

    let mut samples_accumulator = Vec::new();
    let mut resampled_callback = move |written: &[f32]| {
        worker_sender.send(written.len()).unwrap();
        producer.push_slice(written);
    };

    let handle_samples_data = move |samples_frame_data: &[f32]| {
        if samples_frame_data.len() != (target_buffer_size as usize * channels as usize) {
            eprintln!(
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
            Err(e) => eprintln!(
                "resampler error: {e:?}, frames={}, buffer_size={}",
                frames, target_buffer_size
            ),
        }
    };

    let handle_error = move |error| eprintln!("Error: {error}");
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
