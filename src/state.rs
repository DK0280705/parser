use std::{fs::{create_dir, File, OpenOptions}, io::Write, path::Path, sync::Arc};

use dashmap::DashMap;
use serenity::{all::{ChannelId, GuildId}, async_trait};
use songbird::{Event, EventContext, EventHandler as VoiceEventHandler};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct VoiceHandler {
    pub file: Arc<Mutex<File>>,
}

impl VoiceHandler {
    pub fn new(channel_id: ChannelId) -> Self {
        const PCM_DIR_PATH: &'static str = "pcm_dir";

        let path = Path::new(PCM_DIR_PATH);
        if !path.exists() && !path.is_dir() {
            create_dir(PCM_DIR_PATH).expect("Failed to create file");
        }
    
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(format!("{PCM_DIR_PATH}/{channel_id}.pcm"))
            .expect("Failed to create file");
    
        Self {
            file: Arc::new(Mutex::new(file)),
        }
    }
}

#[async_trait]
impl VoiceEventHandler for VoiceHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        match ctx {
            EventContext::VoiceTick(tick) => {
                // This verbosity is educational purpose.
                // The length of the voice samples:
                // num_samples = duration_s * sample_rate_Hz * num_channels
                const SAMPLE_RATE: usize = 48; // kHz
                const DURATION_MS: usize = 20; // voice data duration in ms
                const CHANNEL_SIZE: usize = 2; // 2 channel audio
                const SAMPLE_LENGTH: usize = SAMPLE_RATE * DURATION_MS * CHANNEL_SIZE;

                let mut voice_data = vec![0i32; SAMPLE_LENGTH];
                
                for (_, data) in &tick.speaking {
                    let decoded_voice = data.decoded_voice.as_ref().unwrap();
                    for (sample1, sample2) in voice_data.iter_mut().zip(decoded_voice.iter()) {
                        *sample1 += *sample2 as i32;
                    }
                }

                let transformed_data: Vec<u8> = voice_data.into_iter()
                    .flat_map(|data| {
                        let divisor = data.abs() / i16::MAX as i32 + 1;
                        ((data / divisor) as i16).to_le_bytes()
                    })
                    .collect();
                {
                    let mut file = self.file.lock().await;
                    let _ = file.write_all(&transformed_data);
                }
            }
            _ => {},
        }
        None
    }
}

pub struct State {
    pub record_channels: DashMap<GuildId, VoiceHandler>,
}

impl State {
    pub fn new() -> Self {
        Self {
            record_channels: DashMap::new(),
        }
    }
}
