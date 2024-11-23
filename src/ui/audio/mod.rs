use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, SizedSample};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tracing;

mod connection;
use connection::AudioConnection;

const SILENCE_THRESHOLD: f32 = 0.01; // Adjust this value based on testing
const MIN_CHUNK_DURATION: Duration = Duration::from_millis(500); // Minimum chunk size
const MAX_CHUNK_DURATION: Duration = Duration::from_secs(5); // Maximum chunk size
const SAMPLE_RATE: u32 = 44100;

pub struct AudioCapture {
    is_recording: Arc<AtomicBool>,
    stream: Option<cpal::Stream>,
    audio_connection: Option<AudioConnection>,
    runtime: Runtime,
    chunk_start: Arc<Mutex<Option<Instant>>>,
    buffer: Arc<Mutex<Vec<f32>>>,
    silence_counter: Arc<AtomicUsize>,
}

impl AudioCapture {
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");
        
        tracing::info!("Initializing audio capture and connection...");
        let audio_connection = runtime
            .block_on(async { 
                match AudioConnection::new().await {
                    Ok(conn) => {
                        tracing::info!("Successfully created audio connection");
                        Some(conn)
                    }
                    Err(e) => {
                        tracing::error!("Failed to create audio connection: {}", e);
                        None
                    }
                }
            });

        AudioCapture {
            is_recording: Arc::new(AtomicBool::new(false)),
            stream: None,
            audio_connection,
            runtime,
            chunk_start: Arc::new(Mutex::new(None)),
            buffer: Arc::new(Mutex::new(Vec::new())),
            silence_counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn toggle_recording(&mut self) -> bool {
        let currently_recording = self.is_recording.load(Ordering::SeqCst);
        if currently_recording {
            self.stop_recording();
        } else {
            self.start_recording();
        }
        !currently_recording
    }

    fn start_recording(&mut self) {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .expect("Failed to get default input device");

        let config = device
            .default_input_config()
            .expect("Failed to get default input config");

        let is_recording = self.is_recording.clone();
        is_recording.store(true, Ordering::SeqCst);

        let stream = match config.sample_format() {
            SampleFormat::F32 => self.build_stream::<f32>(&device, &config.into()),
            SampleFormat::I16 => self.build_stream::<i16>(&device, &config.into()),
            SampleFormat::U16 => self.build_stream::<u16>(&device, &config.into()),
            _ => panic!("Unsupported sample format"),
        }
        .expect("Failed to build stream");

        stream.play().expect("Failed to start audio stream");
        self.stream = Some(stream);
    }

    fn stop_recording(&mut self) {
        self.is_recording.store(false, Ordering::SeqCst);
        self.stream = None;
    }

    fn build_stream<T>(
        &self,
        device: &cpal::Device,
        config: &cpal::StreamConfig,
    ) -> Result<cpal::Stream, cpal::BuildStreamError>
    where
        T: Sample<Float = f32> + SizedSample,
    {
        let is_recording = self.is_recording.clone();
        let audio_connection = self.audio_connection.clone();
        let chunk_start = self.chunk_start.clone();
        let buffer = self.buffer.clone();
        let silence_counter = self.silence_counter.clone();

        let runtime = self.runtime.handle().clone();

        device.build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                if is_recording.load(Ordering::SeqCst) {
                    // Start timing if this is the beginning of a chunk
                    let mut chunk_start_guard = chunk_start.lock().unwrap();
                    if chunk_start_guard.is_none() {
                        *chunk_start_guard = Some(Instant::now());
                    }
                    let chunk_started = chunk_start_guard.unwrap();
                    drop(chunk_start_guard);

                    // Convert samples to f32 and check for silence
                    let mut is_silence = true;
                    let samples: Vec<f32> = data
                        .iter()
                        .map(|sample| {
                            let value: f32 = sample.to_float_sample();
                            if value.abs() > SILENCE_THRESHOLD {
                                is_silence = false;
                            }
                            value
                        })
                        .collect();

                    // Update silence counter
                    if is_silence {
                        let _new_count = silence_counter.fetch_add(1, Ordering::SeqCst);
                    } else {
                        silence_counter.store(0, Ordering::SeqCst);
                    }

                    // Add samples to buffer
                    let mut buffer_guard = buffer.lock().unwrap();
                    let _prev_size = buffer_guard.len();
                    buffer_guard.extend(samples);

                    // Check if we should send the chunk
                    let elapsed = chunk_started.elapsed();
                    let silence_count = silence_counter.load(Ordering::SeqCst);
                    let should_send = elapsed >= MAX_CHUNK_DURATION
                        || (elapsed >= MIN_CHUNK_DURATION
                            && silence_count > (SAMPLE_RATE as usize / 10));

                    if should_send {
                        tracing::debug!(
                            "Chunk status: elapsed={:?}, silence_count={}, should_send={}",
                            elapsed,
                            silence_count,
                            should_send
                        );
                    }
                    if should_send {
                        let bytes: Vec<u8> = buffer_guard
                            .drain(..)
                            .flat_map(|sample| sample.to_le_bytes().to_vec())
                            .collect();

                        tracing::info!(
                            "Sending audio chunk: {} samples ({} bytes)",
                            bytes.len() / 4, // 4 bytes per f32
                            bytes.len()
                        );

                        if let Some(connection) = &audio_connection {
                            // Use the runtime to send the audio data
                            runtime.block_on(async {
                                match connection.send_audio(&bytes).await {
                                    Ok(_) => {
                                        tracing::info!("Successfully sent audio chunk");
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to send audio: {}", e);
                                    }
                                }
                            });
                        } else {
                            tracing::error!("No audio connection available!");
                        }

                        // Reset timing
                        let mut chunk_start_guard = chunk_start.lock().unwrap();
                        *chunk_start_guard = None;
                        silence_counter.store(0, Ordering::SeqCst);
                        tracing::debug!("Reset chunk timing and silence counter");
                    }
                }
            },
            move |err| {
                eprintln!("Error in audio stream: {}", err);
            },
            None,
        )
    }
}
