use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct AudioCapture {
    is_recording: Arc<AtomicBool>,
    stream: Option<cpal::Stream>,
}

impl AudioCapture {
    pub fn new() -> Self {
        AudioCapture {
            is_recording: Arc::new(AtomicBool::new(false)),
            stream: None,
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
        let device = host.default_input_device()
            .expect("Failed to get default input device");

        let config = device.default_input_config()
            .expect("Failed to get default input config");

        let is_recording = self.is_recording.clone();
        is_recording.store(true, Ordering::SeqCst);

        let stream = match config.sample_format() {
            SampleFormat::F32 => self.build_stream::<f32>(&device, &config.into()),
            SampleFormat::I16 => self.build_stream::<i16>(&device, &config.into()),
            SampleFormat::U16 => self.build_stream::<u16>(&device, &config.into()),
            _ => panic!("Unsupported sample format"),
        }.expect("Failed to build stream");

        stream.play().expect("Failed to start audio stream");
        self.stream = Some(stream);
    }

    fn stop_recording(&mut self) {
        self.is_recording.store(false, Ordering::SeqCst);
        self.stream = None;
    }

    fn build_stream<T>(&self, device: &cpal::Device, config: &cpal::StreamConfig) 
        -> Result<cpal::Stream, cpal::BuildStreamError> 
        where T: Sample + cpal::SizedSample
    {
        let is_recording = self.is_recording.clone();
        
        device.build_input_stream(
            config,
            move |_data: &[T], _: &cpal::InputCallbackInfo| {
                if is_recording.load(Ordering::SeqCst) {
                    // Here we'll process the audio data
                    // For now, we're just capturing it
                    // TODO: Send this data through the connection
                }
            },
            move |err| {
                eprintln!("Error in audio stream: {}", err);
            },
            None
        )
    }
}
