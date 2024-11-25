use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio_uring::net::UdpSocket;
use std::sync::Arc;
use miette::IntoDiagnostic;
use std::fs::File;
use std::io::Write;
use std::path::Path;

const MAX_UDP_PACKET_SIZE: usize = 1200;
const SAMPLE_RATE: u32 = 44100;
const MIN_AUDIO_DURATION: f32 = 2.0; // Minimum seconds of audio to save

pub struct AudioChunk {
    samples: Vec<f32>,
    last_update: Instant,
}

pub struct AudioProcessor {
    chunks: HashMap<SocketAddr, AudioChunk>,
    socket: Arc<UdpSocket>,
    recording_counter: usize,
}

impl AudioProcessor {
    pub fn new(socket: Arc<UdpSocket>) -> Self {
        Self {
            chunks: HashMap::new(),
            socket,
            recording_counter: 0,
        }
    }

    pub async fn process_packet(&mut self, addr: SocketAddr, data: Vec<u8>) -> miette::Result<()> {
        // Convert bytes back to f32 samples
        let new_samples: Vec<f32> = data.chunks(4)
            .filter_map(|chunk| {
                if chunk.len() == 4 {
                    let arr: [u8; 4] = chunk.try_into().ok()?;
                    Some(f32::from_le_bytes(arr))
                } else {
                    None
                }
            })
            .collect();

        // Get or create chunk for this address
        let chunk = self.chunks.entry(addr).or_insert_with(|| AudioChunk {
            samples: Vec::new(),
            last_update: Instant::now(),
        });
        
        // Add new samples to the chunk
        chunk.samples.extend(new_samples);
        chunk.last_update = Instant::now();

        let duration = chunk.samples.len() as f32 / SAMPLE_RATE as f32;
        
        // Debug audio statistics and save WAV when we have enough data
        if duration >= MIN_AUDIO_DURATION {
            let max_amplitude = chunk.samples.iter().map(|s| s.abs()).fold(0f32, f32::max);
            let avg_amplitude: f32 = chunk.samples.iter().map(|s| s.abs()).sum::<f32>() / chunk.samples.len() as f32;
            
            tracing::info!(
                "Audio stats - Samples: {}, Max amplitude: {:.6}, Avg amplitude: {:.6}",
                chunk.samples.len(),
                max_amplitude,
                avg_amplitude
            );
            // Save the audio chunk as WAV
            let samples = std::mem::take(&mut chunk.samples);
            self.save_wav_file(addr, samples)?;
        }

        Ok(())
    }

    fn save_wav_file(&mut self, addr: SocketAddr, samples: Vec<f32>) -> miette::Result<()> {
        // Create recordings directory if it doesn't exist
        let recordings_dir = Path::new("recordings");
        std::fs::create_dir_all(recordings_dir).into_diagnostic()?;

        // Generate filename with timestamp
        let filename = recordings_dir.join(format!(
            "recording_{:03}_{}.wav",
            self.recording_counter,
            addr.port()
        ));
        self.recording_counter += 1;

        let file = File::create(&filename).into_diagnostic()?;
        let mut writer = std::io::BufWriter::new(file);

        // Write WAV header
        writer.write_all(b"RIFF").into_diagnostic()?;
        let size = 36 + (samples.len() * 4) as u32;
        writer.write_all(&size.to_le_bytes()).into_diagnostic()?;
        writer.write_all(b"WAVE").into_diagnostic()?;
        writer.write_all(b"fmt ").into_diagnostic()?;
        writer.write_all(&16u32.to_le_bytes()).into_diagnostic()?; // Subchunk1Size
        writer.write_all(&1u16.to_le_bytes()).into_diagnostic()?;  // AudioFormat (PCM)
        writer.write_all(&1u16.to_le_bytes()).into_diagnostic()?;  // NumChannels (Mono)
        writer.write_all(&SAMPLE_RATE.to_le_bytes()).into_diagnostic()?;
        writer.write_all(&(SAMPLE_RATE * 4).to_le_bytes()).into_diagnostic()?; // ByteRate
        writer.write_all(&4u16.to_le_bytes()).into_diagnostic()?;  // BlockAlign
        writer.write_all(&32u16.to_le_bytes()).into_diagnostic()?; // BitsPerSample
        writer.write_all(b"data").into_diagnostic()?;
        writer.write_all(&(samples.len() * 4).to_le_bytes()).into_diagnostic()?;

        // Write samples
        for sample in samples {
            writer.write_all(&sample.to_le_bytes()).into_diagnostic()?;
        }

        tracing::info!("Saved WAV file: {:?}", filename);
        Ok(())
    }

    pub fn cleanup_old_chunks(&mut self) {
        const MAX_AGE: Duration = Duration::from_secs(30);
        self.chunks.retain(|_, chunk| {
            chunk.last_update.elapsed() < MAX_AGE
        });
    }
} 