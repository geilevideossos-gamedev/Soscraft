use std::io::Cursor;
use std::num::NonZero;
use std::sync::Arc;
use std::time::Duration;

use bevy::audio::{
    AddAudioSource, AudioSink, AudioSinkPlayback, Decodable, PlaybackSettings, Volume,
};
use bevy::prelude::*;
use rodio::{ChannelCount, SampleRate, Source};
use symphonia::core::audio::{SampleBuffer, SignalSpec};
use symphonia::core::codecs::{CODEC_TYPE_NULL, Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

#[derive(Clone, Debug)]
pub struct McMusicTrack {
    pub key: String,
    pub bytes: Arc<[u8]>,
    pub volume: f32,
}

#[derive(Resource, Default)]
pub struct McMusic {
    pub wanted: Option<McMusicTrack>,
    pub finished: Option<String>,
}

#[derive(Asset, TypePath)]
pub(crate) struct McMusicStream(Arc<[u8]>);

impl Decodable for McMusicStream {
    type Decoder = McMusicDecoder;

    fn decoder(&self) -> Self::Decoder {
        McMusicDecoder::new(Arc::clone(&self.0))
    }
}

struct Stream {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track: u32,
}

pub(crate) struct McMusicDecoder {
    stream: Option<Stream>,
    buffer: Option<(SampleBuffer<f32>, u64, SignalSpec)>,
    next: usize,
    channels: ChannelCount,
    sample_rate: SampleRate,
}

fn open(bytes: Arc<[u8]>) -> Option<Stream> {
    let source = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
    let probed = symphonia::default::get_probe()
        .format(
            &Hint::new(),
            source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;
    let format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)?;
    let decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .ok()?;
    let track = track.id;
    Some(Stream {
        format,
        decoder,
        track,
    })
}

impl McMusicDecoder {
    pub(crate) fn new(bytes: Arc<[u8]>) -> Self {
        let mut decoder = Self {
            stream: open(bytes),
            buffer: None,
            next: 0,
            channels: NonZero::new(2).unwrap(),
            sample_rate: NonZero::new(44_100).unwrap(),
        };
        if decoder.refill()
            && let Some((_, _, spec)) = &decoder.buffer
        {
            let channels = u16::try_from(spec.channels.count()).ok();
            if let Some(channels) = channels.and_then(NonZero::new) {
                decoder.channels = channels;
            }
            if let Some(rate) = NonZero::new(spec.rate) {
                decoder.sample_rate = rate;
            }
        }
        decoder
    }

    fn refill(&mut self) -> bool {
        let Self {
            stream,
            buffer,
            next,
            ..
        } = self;
        let Some(open) = stream.as_mut() else {
            return false;
        };
        loop {
            let packet = match open.format.next_packet() {
                Ok(packet) => packet,
                Err(symphonia::core::errors::Error::ResetRequired) => continue,
                Err(_) => break,
            };
            if packet.track_id() != open.track {
                continue;
            }
            let Ok(decoded) = open.decoder.decode(&packet) else {
                continue;
            };
            let spec = *decoded.spec();
            let frames = decoded.capacity() as u64;
            if buffer.as_ref().is_some_and(|(_, _, first)| {
                first.channels != spec.channels || first.rate != spec.rate
            }) {
                break;
            }
            if buffer.as_ref().is_none_or(|(_, room, _)| *room < frames) {
                *buffer = Some((SampleBuffer::new(frames, spec), frames, spec));
            }
            if let Some((samples, _, _)) = buffer.as_mut() {
                samples.copy_interleaved_ref(decoded);
            }
            *next = 0;
            return true;
        }
        *stream = None;
        false
    }
}

impl Iterator for McMusicDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((samples, _, _)) = &self.buffer
                && let Some(&sample) = samples.samples().get(self.next)
            {
                self.next += 1;
                return Some(sample);
            }
            if !self.refill() {
                return None;
            }
        }
    }
}

impl Source for McMusicDecoder {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        self.channels
    }

    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

fn play_minecraft_music(
    mut music: ResMut<McMusic>,
    mut playing: Local<Option<(String, Entity)>>,
    mut commands: Commands,
    mut streams: ResMut<Assets<McMusicStream>>,
    mut voices: Query<Option<&mut AudioSink>, With<AudioPlayer<McMusicStream>>>,
    settings: Option<Res<frame::GameSettings>>,
) {
    let master = settings.map_or(1.0, |settings| settings.master_volume);
    let gain = |track: &McMusicTrack| (track.volume * crate::minecraft::MIX_GAIN * master).max(0.0);
    if let Some((key, entity)) = playing.clone() {
        let ended = match voices.get(entity) {
            Ok(Some(sink)) => sink.empty(),
            Ok(None) => false,
            Err(_) => true,
        };
        let still_wanted = music
            .wanted
            .as_ref()
            .filter(|track| track.key == key)
            .map(gain);
        if ended {
            commands.entity(entity).try_despawn();
            *playing = None;
            if still_wanted.is_some() {
                music.wanted = None;
            }
            music.finished = Some(key);
        } else if let Some(volume) = still_wanted {
            if let Ok(Some(mut sink)) = voices.get_mut(entity) {
                sink.set_volume(Volume::Linear(volume));
            }
        } else {
            commands.entity(entity).try_despawn();
            *playing = None;
        }
    }
    if playing.is_none()
        && let Some(track) = music.wanted.as_ref()
    {
        let stream = streams.add(McMusicStream(Arc::clone(&track.bytes)));
        let entity = commands
            .spawn((
                AudioPlayer(stream),
                PlaybackSettings::DESPAWN.with_volume(Volume::Linear(gain(track))),
            ))
            .id();
        *playing = Some((track.key.clone(), entity));
    }
}

pub(crate) fn register(app: &mut App) {
    app.init_resource::<McMusic>()
        .add_audio_source::<McMusicStream>()
        .add_systems(Update, play_minecraft_music);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_music_stream_decodes_the_same_samples_as_the_whole_file_decoder() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let file = std::path::Path::new(&root).join(
            "resourcepacks/local/minecraft-26.3/assets/minecraft/sounds/ambient/weather/end_flash1.ogg",
        );
        let Ok(bytes) = std::fs::read(file) else {
            return;
        };
        let whole = crate::pcm::decode_audio_bytes(&bytes).unwrap();
        let stream = McMusicDecoder::new(bytes.into());
        assert_eq!(stream.channels().get(), whole.channel_count());
        assert_eq!(stream.sample_rate().get(), whole.rate());
        let samples: Vec<f32> = stream.collect();
        assert_eq!(samples.len(), whole.samples().len());
        assert!(samples.len() > 1000);
        assert_eq!(samples[..], whole.samples()[..]);
    }

    #[test]
    fn a_stream_that_is_not_ogg_ends_at_once() {
        let mut stream = McMusicDecoder::new(Arc::from(&b"not an ogg file"[..]));
        assert_eq!(stream.next(), None);
        assert_eq!(stream.channels().get(), 2);
    }
}
