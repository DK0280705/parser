use std::io::{Cursor, Read, Seek, SeekFrom};

use crate::state::{State, VoiceHandler};
use hound::{WavSpec, WavWriter};
use serenity::all::{
    ChannelType, CommandInteraction, CommandOptionType, Context, CreateAttachment, CreateCommand, CreateCommandOption, CreateInteractionResponse, CreateInteractionResponseMessage, ResolvedValue
};
use songbird::CoreEvent;

use super::reply;

pub async fn run(
    state: &State,
    ctx: &Context,
    itr: &CommandInteraction,
) -> Result<(), serenity::Error> {
    let opts = itr.data.options();
    let first_opt = opts.first().unwrap();
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird voice client placed in at initialization")
        .clone();

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
                let mut pcm_file = voice_handler.file.lock().await;
                pcm_file.seek(SeekFrom::Start(0)).expect("Can't even seek in a file");
                pcm_file.read_to_end(&mut pcm_data)
            };
            state.record_channels.remove(&guild_id);
            
            if let Err(err) = file_read_res {
                eprintln!("Failed to read pcm data: {err:?}");
                return reply(ctx, itr, "Failed to read recording file!").await;
            }

            match create_wav(&pcm_data) {
                Ok(wav_data) => {
                    itr.create_response(&ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content("Recording stopped")
                                .add_file(CreateAttachment::bytes(wav_data, "Recording.wav"))
                        )
                    ).await
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
        let mut writer = WavWriter::new(Cursor::new(&mut wav_data), WavSpec {
            channels: 2,
            sample_rate: 48000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int
        })?;
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
