# subwin

Captures system audio and transcribes speech into live captions using OpenAI's Whisper. This project is Work-In-Progress (WIP).

## Features

* Real-time audio capture for system audio
* Streaming speech transcription/translation with Whisper
* ~1 second end-to-end latency target

## Audio Pipeline Overview

In order to get a reliable and low-latency transcription, we've built a custom audio pipeline.

### Current pipeline:

```
CPAL (audio input)
  ↓
Stereo → Mono mixing
  ↓
Resampler (fixed-block or streaming)
  ↓
Buffered mono audio @ 16 kHz
  ↓
Transcription
```

### Audio Capture

* Audio input is handled using [CPAL](https://github.com/RustAudio/cpal)
* If possible, a fixed buffer size is requested from the audio backend

### Resampling Strategy

Since we want to transcript the audio later, we need to downsample and remix the input audio. The goal is to get audio at 16 kHz with a single (mono) channel. For this purpose there's **two resamplers**, both based on `rubato::FftFixedInOut`:

#### FixedBlockResampler

* Zero buffering
* Lowest latency
* Requires:
  * A fixed input buffer size
  * The buffer size to be mathematically compatible with the sample-rate ratio
* Used **only when the device buffer size allows it**

#### StreamingResampler (FIFO-based)

* Accepts arbitrary input buffer sizes
* Internally buffers input until the resampler's required block size is met
* Slightly higher latency
* Works as a fallback method of resampling

### Automatic Selection

At runtime, the program:

1. Inspects the device's supported buffer size range
2. Checks if a compatible fixed block size exists
3. Uses `FixedBlockResampler` if possible
4. Falls back to `StreamingResampler` otherwise

This avoids subtle off-by-one errors and timing drift.
