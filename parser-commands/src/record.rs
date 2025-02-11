use std::{
    fs::{File, OpenOptions},
    io::{Cursor, Read, Seek, SeekFrom, Write},
    path::Path,
    sync::{Arc, Mutex},
};

use hound::{WavSpec, WavWriter};
use serenity::{
    all::{
        ChannelId, ChannelType, CommandInteraction, CommandOptionType, Context, CreateAttachment,
        CreateCommand, CreateCommandOption, CreateInteractionResponse,
        CreateInteractionResponseMessage, ResolvedValue,
    },
    async_trait,
};
use songbird::{CoreEvent, Event, EventContext, EventHandler as VoiceEventHandler};

use super::reply;
use crate::State;

#[derive(Clone)]
pub struct VoiceHandler {
    pub file: Arc<Mutex<File>>,
}

impl VoiceHandler {
    pub fn new(channel_id: ChannelId) -> Self {
        const PCM_DIR_PATH: &'static str = "pcm_dir";

        let path = Path::new(PCM_DIR_PATH);
        if !path.exists() && !path.is_dir() {
            std::fs::create_dir(PCM_DIR_PATH).expect("Failed to create file");
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

                let transformed_data: Vec<u8> = voice_data
                    .into_iter()
                    .flat_map(|data| {
                        let divisor = data.abs() / i16::MAX as i32 + 1;
                        ((data / divisor) as i16).to_le_bytes()
                    })
                    .collect();
                {
                    let mut file = self.file.lock().unwrap();
                    let _ = file.write_all(&transformed_data);
                }
            }
            _ => {}
        }
        None
    }
}

pub async fn run(
    state: &State,
    ctx: &Context,
    itr: &CommandInteraction,
) -> Result<(), serenity::Error> {
    let opts = itr.data.options();
    let first_opt = opts.first().unwrap();
    let ref manager = state.voice_manager;

    let guild_id = itr.guild_id.unwrap();

    match first_opt.name {
        "start" => {
            if manager.get(guild_id).is_some() {
                return reply(ctx, itr, "You need to stop the recording first").await;
            }

            let ResolvedValue::SubCommand(ref sub_command) = first_opt.value else {
                panic!("Boom");
            };
            let ResolvedValue::Channel(channel) = sub_command.first().unwrap().value else {
                panic!("Boom");
            };
            assert_eq!(channel.kind, ChannelType::Voice);

            let Ok(call_lock) = manager.join(guild_id, channel.id).await else {
                return reply(ctx, itr, "Can't connect to voice channel!").await;
            };

            {
                let mut call = call_lock.lock().await;
                let vc_entry = state
                    .record_channels
                    .entry(guild_id)
                    .insert_entry(VoiceHandler::new(channel.id));
                call.add_global_event(CoreEvent::VoiceTick.into(), vc_entry.get().clone());
            }

            reply(ctx, itr, "Recording started!").await
        }
        "stop" => {
            if manager.get(guild_id).is_none() {
                return reply(ctx, itr, "Already stopped").await;
            }

            manager
                .remove(guild_id)
                .await
                .expect("The songbird is doomed");

            let mut pcm_data: Vec<u8> = Vec::new();

            let file_read_res = {
                let voice_handler = state.record_channels.get(&guild_id).unwrap();
                let mut pcm_file = voice_handler.file.lock().unwrap();
                pcm_file
                    .seek(SeekFrom::Start(0))
                    .expect("Can't even seek in a file");
                pcm_file.read_to_end(&mut pcm_data)
            };
            state.record_channels.remove(&guild_id);

            if let Err(err) = file_read_res {
                eprintln!("Failed to read pcm data: {err:?}");
                return reply(ctx, itr, "Failed to read recording file!").await;
            }

            match create_wav(&pcm_data) {
                Ok(wav_data) => {
                    itr.create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content("Recording stopped")
                                .add_file(CreateAttachment::bytes(wav_data, "Recording.wav")),
                        ),
                    )
                    .await
                }
                Err(err) => {
                    eprintln!("Failed to create wav file: {err}");
                    reply(ctx, itr, "Failed to create wav file!").await
                }
            }
        }
        _ => unreachable!("Ain't no way unless i'm dumb"),
    }
}

fn create_wav(pcm_data: &Vec<u8>) -> Result<Vec<u8>, hound::Error> {
    let mut wav_data: Vec<u8> = Vec::new();
    {
        let mut writer = WavWriter::new(
            Cursor::new(&mut wav_data),
            WavSpec {
                channels: 2,
                sample_rate: 48000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )?;
        for chunk in pcm_data.chunks_exact(2) {
            writer.write_sample(i16::from_le_bytes([chunk[0], chunk[1]]))?;
        }
        writer.finalize()?;
    }
    Ok(wav_data)
}

pub fn create() -> CreateCommand {
    CreateCommand::new("record")
        .description("Join a voice channel")
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "start", "Start recording")
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Channel,
                        "channel",
                        "Channel to join",
                    )
                    .channel_types(vec![ChannelType::Voice])
                    .required(true),
                ),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "stop",
            "Stop recording",
        ))
}
