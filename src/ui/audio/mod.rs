use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample};
use std::fs::File;
use std::io::BufWriter;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use tracing;

mod connection;
use connection::AudioConnection;
use chrono::Local;

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
    wav_writer: WavWriterHandle,
}

type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

impl AudioCapture {
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");

        tracing::info!("Initializing audio capture and connection...");
        let audio_connection = runtime.block_on(async {
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
            wav_writer: Arc::new(Mutex::new(None)),
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

        let current_datetime = Local::now();
        let formatted_datetime:String = current_datetime.format("%Y-%m-%d-%H:%M:%S").to_string();
        let path = format!("{}/recordings/record_{}.wav", env!("CARGO_MANIFEST_DIR"), formatted_datetime);
        let spec = self.wav_spec_from_config(&config);
        let writer = hound::WavWriter::create(&path, spec).unwrap();
        let writer = Arc::new(Mutex::new(Some(writer)));

        self.wav_writer = writer;

        let is_recording = self.is_recording.clone();
        is_recording.store(true, Ordering::SeqCst);

        let stream = match config.sample_format() {
            SampleFormat::F32 => self.build_stream::<f32>(&device, &config.into()),
            SampleFormat::I8 => self.build_stream::<i8>(&device, &config.into()),
            SampleFormat::I16 => self.build_stream::<i16>(&device, &config.into()),
            SampleFormat::I32 => self.build_stream::<i32>(&device, &config.into()),
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
        T: Sample<Float = f32> +hound::Sample+ SizedSample,
    {
        let is_recording = self.is_recording.clone();
        let writer = self.wav_writer.clone();
        // let audio_connection = self.audio_connection.clone();
        // let chunk_start = self.chunk_start.clone();
        // let buffer = self.buffer.clone();
        // let silence_counter = self.silence_counter.clone();

        // let runtime = self.runtime.handle().clone();

        device.build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                if !is_recording.load(Ordering::SeqCst) {
                    return;
                }

                write_input_data::<T, T>(data, &writer);
            },
            move |err| {
                eprintln!("Error in audio stream: {}", err);
            },
            None,
        )
    }



    fn sample_format(&self, format: cpal::SampleFormat) -> hound::SampleFormat {
        if format.is_float() {
            hound::SampleFormat::Float
        } else {
            hound::SampleFormat::Int
        }
    }

    fn wav_spec_from_config(&self, config: &cpal::SupportedStreamConfig) -> hound::WavSpec {
        hound::WavSpec {
            channels: config.channels() as _,
            sample_rate: config.sample_rate().0 as _,
            bits_per_sample: (config.sample_format().sample_size() * 8) as _,
            sample_format: self.sample_format(config.sample_format()),
        }
    }
}

fn write_input_data<T, U>( input: &[T], writer: &WavWriterHandle)
where
    T: Sample,
    U: Sample + hound::Sample + FromSample<T>,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for &sample in input.iter() {
                let sample: U = U::from_sample(sample);
                writer.write_sample(sample).ok();
            }
        }
    }
}