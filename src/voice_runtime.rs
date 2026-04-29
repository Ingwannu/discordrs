use std::collections::HashMap;
#[cfg(feature = "dave")]
use std::num::NonZeroU16;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use aes_gcm::aead::{AeadInPlace, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce as AesNonce};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use futures_util::{SinkExt, StreamExt};
use opus_decoder::OpusDecoder;
#[cfg(feature = "voice-encode")]
use opus_rs::{Application as OpusApplication, OpusEncoder as RawOpusEncoder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, oneshot, watch, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

use crate::error::DiscordError;
use crate::model::Snowflake;
use crate::types::invalid_data_error;
use crate::voice::{
    VoiceEncryptionMode, VoiceGatewayCommand, VoiceGatewayOpcode, VoiceGatewayReady,
    VoiceSelectProtocolCommand, VoiceSpeakingCommand, VoiceSpeakingFlags, VoiceUdpDiscoveryPacket,
};

const VOICE_OP_READY: u64 = 2;
const VOICE_OP_HEARTBEAT: u64 = 3;
const VOICE_OP_SESSION_DESCRIPTION: u64 = 4;
const VOICE_OP_HEARTBEAT_ACK: u64 = 6;
const VOICE_OP_RESUME: u64 = 7;
const VOICE_OP_HELLO: u64 = 8;
const VOICE_OP_RESUMED: u64 = 9;
const VOICE_OP_CLIENTS_CONNECT: u64 = 11;
const VOICE_OP_CLIENT_DISCONNECT: u64 = 13;
const VOICE_OP_DAVE_PREPARE_TRANSITION: u64 = 21;
const VOICE_OP_DAVE_EXECUTE_TRANSITION: u64 = 22;
const VOICE_OP_DAVE_PREPARE_EPOCH: u64 = 24;
const VOICE_OP_DAVE_MLS_EXTERNAL_SENDER: u64 = 25;
const VOICE_OP_DAVE_MLS_PROPOSALS: u64 = 27;
const VOICE_OP_DAVE_MLS_ANNOUNCE_COMMIT_TRANSITION: u64 = 29;
const VOICE_OP_DAVE_MLS_WELCOME: u64 = 30;
const DAVE_MAGIC_MARKER: [u8; 2] = [0xfa, 0xfa];
const RTP_VERSION: u8 = 2;
const RTP_PAYLOAD_TYPE_OPUS: u8 = 120;
const DISCORD_OPUS_SAMPLE_RATE: u32 = 48_000;
const DISCORD_OPUS_CHANNELS: usize = 2;
const DISCORD_OPUS_SAMPLES_PER_CHANNEL: usize = 960;
const DISCORD_OPUS_STEREO_FRAME_SAMPLES: usize =
    DISCORD_OPUS_CHANNELS * DISCORD_OPUS_SAMPLES_PER_CHANNEL;
const DISCORD_OPUS_FRAME_MS: u64 = 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceRuntimeConfig {
    pub server_id: Snowflake,
    pub user_id: Snowflake,
    pub session_id: String,
    pub token: String,
    pub endpoint: String,
    pub gateway_version: u8,
    pub preferred_mode: Option<VoiceEncryptionMode>,
    pub max_dave_protocol_version: Option<u8>,
}

impl VoiceRuntimeConfig {
    pub fn new(
        server_id: impl Into<Snowflake>,
        user_id: impl Into<Snowflake>,
        session_id: impl Into<String>,
        token: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Self {
        Self {
            server_id: server_id.into(),
            user_id: user_id.into(),
            session_id: session_id.into(),
            token: token.into(),
            endpoint: endpoint.into(),
            gateway_version: 8,
            preferred_mode: None,
            max_dave_protocol_version: Some(1),
        }
    }

    pub fn gateway_version(mut self, gateway_version: u8) -> Self {
        self.gateway_version = gateway_version.max(4);
        self
    }

    pub fn preferred_mode(mut self, preferred_mode: VoiceEncryptionMode) -> Self {
        self.preferred_mode = Some(preferred_mode);
        self
    }

    pub fn max_dave_protocol_version(mut self, version: u8) -> Self {
        self.max_dave_protocol_version = Some(version);
        self
    }

    pub fn websocket_url(&self) -> String {
        let mut endpoint = if self.endpoint.contains("://") {
            self.endpoint.clone()
        } else {
            format!("wss://{}", self.endpoint)
        };

        if !endpoint.contains("?v=") {
            let separator = if endpoint.contains('?') { "&" } else { "/?" };
            endpoint.push_str(separator);
            endpoint.push_str(&format!("v={}", self.gateway_version.max(4)));
        }

        endpoint
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionDescription {
    pub mode: VoiceEncryptionMode,
    #[serde(default)]
    pub secret_key: Vec<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_codec: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dave_protocol_version: Option<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct VoiceDaveState {
    pub protocol_version: Option<u8>,
    pub transition_id: Option<u64>,
    pub epoch: Option<u64>,
    pub passthrough: bool,
    #[serde(default)]
    pub external_sender: Option<Vec<u8>>,
    #[serde(default)]
    pub proposals: Vec<Vec<u8>>,
    #[serde(default)]
    pub pending_commit: Option<Vec<u8>>,
    #[serde(default)]
    pub pending_welcome: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceRuntimeState {
    pub config: VoiceRuntimeConfig,
    pub heartbeat_interval_ms: u64,
    pub last_sequence: Option<i64>,
    pub ready: VoiceGatewayReady,
    pub discovery: VoiceUdpDiscoveryPacket,
    pub selected_mode: VoiceEncryptionMode,
    pub session_description: Option<VoiceSessionDescription>,
    pub ssrc_users: HashMap<u32, Snowflake>,
    pub speaking: HashMap<u32, VoiceSpeakingUpdate>,
    pub dave: VoiceDaveState,
    pub resumed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceRawUdpPacket {
    pub bytes: Vec<u8>,
    pub version: Option<u8>,
    pub payload_type: Option<u8>,
    pub sequence: Option<u16>,
    pub timestamp: Option<u32>,
    pub ssrc: Option<u32>,
}

impl VoiceRawUdpPacket {
    fn from_bytes(bytes: Vec<u8>) -> Self {
        let (version, payload_type, sequence, timestamp, ssrc) = if bytes.len() >= 12 {
            (
                Some(bytes[0] >> 6),
                Some(bytes[1] & 0x7f),
                Some(u16::from_be_bytes([bytes[2], bytes[3]])),
                Some(u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]])),
                Some(u32::from_be_bytes([
                    bytes[8], bytes[9], bytes[10], bytes[11],
                ])),
            )
        } else {
            (None, None, None, None, None)
        };

        Self {
            bytes,
            version,
            payload_type,
            sequence,
            timestamp,
            ssrc,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceRtpHeader {
    pub version: u8,
    pub padding: bool,
    pub extension: bool,
    pub marker: bool,
    pub payload_type: u8,
    pub sequence: u16,
    pub timestamp: u32,
    pub ssrc: u32,
    pub header_len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceReceivedPacket {
    pub raw: VoiceRawUdpPacket,
    pub rtp: VoiceRtpHeader,
    pub user_id: Option<Snowflake>,
    pub opus_frame: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceDecodedPacket {
    pub packet: VoiceReceivedPacket,
    pub sample_rate: u32,
    pub channels: usize,
    pub samples_per_channel: usize,
    pub pcm: Vec<i16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceOpusFrame {
    pub bytes: Vec<u8>,
    pub duration: Duration,
}

impl VoiceOpusFrame {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            bytes: bytes.into(),
            duration: Duration::from_millis(DISCORD_OPUS_FRAME_MS),
        }
    }

    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }
}

#[cfg(feature = "voice-encode")]
#[derive(Clone, Debug, PartialEq)]
pub struct PcmFrame {
    samples: Vec<f32>,
    sample_rate: u32,
    channels: usize,
    duration: Duration,
}

#[cfg(feature = "voice-encode")]
impl PcmFrame {
    pub fn new(
        samples: impl Into<Vec<f32>>,
        sample_rate: u32,
        channels: usize,
        duration: Duration,
    ) -> Result<Self, DiscordError> {
        let samples = samples.into();
        if sample_rate != DISCORD_OPUS_SAMPLE_RATE {
            return Err(invalid_data_error(format!(
                "PCM frame sample_rate must be {DISCORD_OPUS_SAMPLE_RATE} Hz"
            )));
        }
        if channels != DISCORD_OPUS_CHANNELS {
            return Err(invalid_data_error(format!(
                "PCM frame channels must be {DISCORD_OPUS_CHANNELS}"
            )));
        }
        if duration != Duration::from_millis(DISCORD_OPUS_FRAME_MS) {
            return Err(invalid_data_error(format!(
                "PCM frame duration must be {DISCORD_OPUS_FRAME_MS}ms"
            )));
        }
        if samples.len() != DISCORD_OPUS_STEREO_FRAME_SAMPLES {
            return Err(invalid_data_error(format!(
                "PCM frame must contain {DISCORD_OPUS_STEREO_FRAME_SAMPLES} interleaved f32 samples"
            )));
        }

        Ok(Self {
            samples,
            sample_rate,
            channels,
            duration,
        })
    }

    pub fn discord_stereo_20ms(samples: impl Into<Vec<f32>>) -> Result<Self, DiscordError> {
        Self::new(
            samples,
            DISCORD_OPUS_SAMPLE_RATE,
            DISCORD_OPUS_CHANNELS,
            Duration::from_millis(DISCORD_OPUS_FRAME_MS),
        )
    }

    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn samples_per_channel(&self) -> usize {
        self.samples.len() / self.channels
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }
}

#[cfg(feature = "voice-encode")]
pub trait AudioSource {
    fn next_pcm_frame(&mut self) -> Result<Option<PcmFrame>, DiscordError>;
}

#[cfg(feature = "voice-encode")]
pub struct AudioMixer {
    sources: Vec<Box<dyn AudioSource + Send>>,
    volume: f32,
}

#[cfg(feature = "voice-encode")]
impl AudioMixer {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            volume: 1.0,
        }
    }

    pub fn with_volume(volume: f32) -> Self {
        Self {
            sources: Vec::new(),
            volume,
        }
    }

    pub fn push_source<S>(&mut self, source: S)
    where
        S: AudioSource + Send + 'static,
    {
        self.sources.push(Box::new(source));
    }

    pub fn active_sources(&self) -> usize {
        self.sources.len()
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume;
    }
}

#[cfg(feature = "voice-encode")]
impl Default for AudioMixer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "voice-encode")]
impl AudioSource for AudioMixer {
    fn next_pcm_frame(&mut self) -> Result<Option<PcmFrame>, DiscordError> {
        let mut mixed = vec![0.0_f32; DISCORD_OPUS_STEREO_FRAME_SAMPLES];
        let mut mixed_any = false;
        let mut index = 0;

        while index < self.sources.len() {
            match self.sources[index].next_pcm_frame()? {
                Some(frame) => {
                    if frame.sample_rate() != DISCORD_OPUS_SAMPLE_RATE
                        || frame.channels() != DISCORD_OPUS_CHANNELS
                        || frame.duration() != Duration::from_millis(DISCORD_OPUS_FRAME_MS)
                        || frame.samples().len() != DISCORD_OPUS_STEREO_FRAME_SAMPLES
                    {
                        return Err(invalid_data_error(
                            "audio mixer only accepts 48kHz stereo 20ms PCM frames",
                        ));
                    }
                    for (target, sample) in mixed.iter_mut().zip(frame.samples()) {
                        *target += sample * self.volume;
                    }
                    mixed_any = true;
                    index += 1;
                }
                None => {
                    self.sources.remove(index);
                }
            }
        }

        if !mixed_any {
            return Ok(None);
        }

        for sample in &mut mixed {
            *sample = sample.clamp(-1.0, 1.0);
        }

        PcmFrame::discord_stereo_20ms(mixed).map(Some)
    }
}

#[cfg(feature = "voice-encode")]
pub struct VoiceOpusEncoder {
    encoder: RawOpusEncoder,
    bitrate_bps: i32,
    use_cbr: bool,
    output: Vec<u8>,
}

#[cfg(feature = "voice-encode")]
impl VoiceOpusEncoder {
    pub fn new(bitrate_bps: i32, use_cbr: bool) -> Result<Self, DiscordError> {
        let mut encoder = RawOpusEncoder::new(
            DISCORD_OPUS_SAMPLE_RATE as i32,
            DISCORD_OPUS_CHANNELS,
            OpusApplication::Audio,
        )
        .map_err(|error| invalid_data_error(format!("failed to create Opus encoder: {error}")))?;
        encoder.bitrate_bps = bitrate_bps;
        encoder.use_cbr = use_cbr;

        Ok(Self {
            encoder,
            bitrate_bps,
            use_cbr,
            output: vec![0_u8; 4_096],
        })
    }

    pub fn discord_music() -> Result<Self, DiscordError> {
        Self::new(128_000, true)
    }

    pub fn with_bitrate_bps(mut self, bitrate_bps: i32) -> Self {
        self.encoder.bitrate_bps = bitrate_bps;
        self.bitrate_bps = bitrate_bps;
        self
    }

    pub fn with_cbr(mut self, use_cbr: bool) -> Self {
        self.encoder.use_cbr = use_cbr;
        self.use_cbr = use_cbr;
        self
    }

    pub fn sample_rate(&self) -> u32 {
        DISCORD_OPUS_SAMPLE_RATE
    }

    pub fn channels(&self) -> usize {
        DISCORD_OPUS_CHANNELS
    }

    pub fn samples_per_channel(&self) -> usize {
        DISCORD_OPUS_SAMPLES_PER_CHANNEL
    }

    pub fn bitrate_bps(&self) -> i32 {
        self.bitrate_bps
    }

    pub fn use_cbr(&self) -> bool {
        self.use_cbr
    }

    pub fn encode_pcm_frame(&mut self, frame: &PcmFrame) -> Result<VoiceOpusFrame, DiscordError> {
        if frame.sample_rate() != DISCORD_OPUS_SAMPLE_RATE
            || frame.channels() != DISCORD_OPUS_CHANNELS
            || frame.samples_per_channel() != DISCORD_OPUS_SAMPLES_PER_CHANNEL
            || frame.duration() != Duration::from_millis(DISCORD_OPUS_FRAME_MS)
        {
            return Err(invalid_data_error(
                "Opus encoder requires 48kHz stereo 20ms PCM frames",
            ));
        }

        let written = self
            .encoder
            .encode(
                frame.samples(),
                DISCORD_OPUS_SAMPLES_PER_CHANNEL,
                &mut self.output,
            )
            .map_err(|error| invalid_data_error(format!("failed to encode Opus frame: {error}")))?;
        Ok(VoiceOpusFrame {
            bytes: self.output[..written].to_vec(),
            duration: frame.duration(),
        })
    }

    pub fn encode_source_frames<S>(
        &mut self,
        source: &mut S,
        max_frames: Option<usize>,
    ) -> Result<Vec<VoiceOpusFrame>, DiscordError>
    where
        S: AudioSource + ?Sized,
    {
        let mut frames = Vec::new();
        while max_frames.is_none_or(|max| frames.len() < max) {
            let Some(frame) = source.next_pcm_frame()? else {
                break;
            };
            frames.push(self.encode_pcm_frame(&frame)?);
        }
        Ok(frames)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceOutboundPacket {
    pub rtp: VoiceRtpHeader,
    pub nonce_suffix: [u8; 4],
    pub opus_frame: Vec<u8>,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceOutboundRtpState {
    pub sequence: u16,
    pub timestamp: u32,
    pub nonce_suffix: u32,
    pub ssrc: u32,
    pub payload_type: u8,
    pub sample_rate: u32,
}

impl VoiceOutboundRtpState {
    pub fn new(ssrc: u32) -> Self {
        Self {
            sequence: 0,
            timestamp: 0,
            nonce_suffix: initial_voice_heartbeat_nonce() as u32,
            ssrc,
            payload_type: RTP_PAYLOAD_TYPE_OPUS,
            sample_rate: DISCORD_OPUS_SAMPLE_RATE,
        }
    }

    pub fn with_counters(ssrc: u32, sequence: u16, timestamp: u32, nonce_suffix: u32) -> Self {
        Self {
            sequence,
            timestamp,
            nonce_suffix,
            ssrc,
            payload_type: RTP_PAYLOAD_TYPE_OPUS,
            sample_rate: DISCORD_OPUS_SAMPLE_RATE,
        }
    }

    pub fn build_packet(
        &mut self,
        opus_frame: &[u8],
        duration: Duration,
        mode: &VoiceEncryptionMode,
        secret_key: &[u8],
    ) -> Result<VoiceOutboundPacket, DiscordError> {
        if opus_frame.is_empty() {
            return Err(invalid_data_error("opus frame must not be empty"));
        }

        let sequence = self.sequence;
        let timestamp = self.timestamp;
        let nonce_suffix = self.nonce_suffix.to_be_bytes();
        let packet = encrypt_transport_payload(
            VoiceOutboundEncryptParams {
                sequence,
                timestamp,
                ssrc: self.ssrc,
                payload_type: self.payload_type,
                nonce_suffix,
            },
            opus_frame,
            mode,
            secret_key,
        )?;
        let rtp = parse_rtp_header(&packet)?;

        self.sequence = self.sequence.wrapping_add(1);
        self.timestamp = self
            .timestamp
            .wrapping_add(timestamp_increment(self.sample_rate, duration));
        self.nonce_suffix = self.nonce_suffix.wrapping_add(1);

        Ok(VoiceOutboundPacket {
            rtp,
            nonce_suffix,
            opus_frame: opus_frame.to_vec(),
            bytes: packet,
        })
    }
}

pub struct VoiceOpusDecoder {
    decoder: OpusDecoder,
    sample_rate: u32,
    channels: usize,
    max_samples_per_channel: usize,
}

impl VoiceOpusDecoder {
    pub fn new(sample_rate: u32, channels: usize) -> Result<Self, DiscordError> {
        let decoder = OpusDecoder::new(sample_rate, channels).map_err(|error| {
            invalid_data_error(format!("failed to create Opus decoder: {error}"))
        })?;
        let max_samples_per_channel = decoder.max_frame_size_per_channel();
        Ok(Self {
            decoder,
            sample_rate,
            channels,
            max_samples_per_channel,
        })
    }

    pub fn discord_default() -> Result<Self, DiscordError> {
        Self::new(48_000, 2)
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn decode_opus_frame(
        &mut self,
        opus_frame: &[u8],
        fec: bool,
    ) -> Result<(usize, Vec<i16>), DiscordError> {
        let mut pcm = vec![0_i16; self.max_samples_per_channel * self.channels];
        let samples_per_channel = self
            .decoder
            .decode(opus_frame, &mut pcm, fec)
            .map_err(|error| invalid_data_error(format!("failed to decode Opus frame: {error}")))?;
        pcm.truncate(samples_per_channel * self.channels);
        Ok((samples_per_channel, pcm))
    }

    pub fn decode_packet(
        &mut self,
        packet: VoiceReceivedPacket,
    ) -> Result<VoiceDecodedPacket, DiscordError> {
        let (samples_per_channel, pcm) = self.decode_opus_frame(&packet.opus_frame, false)?;
        Ok(VoiceDecodedPacket {
            packet,
            sample_rate: self.sample_rate,
            channels: self.channels,
            samples_per_channel,
            pcm,
        })
    }

    pub fn reset(&mut self) {
        self.decoder.reset();
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceDaveUnencryptedRange {
    pub offset: u64,
    pub len: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceDaveFrame {
    pub bytes: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub auth_tag: [u8; 8],
    pub nonce: u32,
    pub unencrypted_ranges: Vec<VoiceDaveUnencryptedRange>,
    pub supplemental_size: u8,
}

pub trait VoiceDaveFrameDecryptor {
    fn decrypt_frame(
        &mut self,
        rtp: &VoiceRtpHeader,
        user_id: Option<&Snowflake>,
        frame: &VoiceDaveFrame,
    ) -> Result<Vec<u8>, DiscordError>;
}

#[cfg(feature = "dave")]
pub trait VoiceDaveFrameEncryptor {
    fn encrypt_opus(&mut self, opus_frame: &[u8]) -> Result<Vec<u8>, DiscordError>;
}

#[cfg(feature = "dave")]
pub struct VoiceDaveySession {
    session: davey::DaveSession,
}

#[cfg(feature = "dave")]
pub type VoiceDaveyDecryptor = VoiceDaveySession;

#[cfg(feature = "dave")]
impl VoiceDaveySession {
    pub fn new(
        protocol_version: NonZeroU16,
        user_id: u64,
        channel_id: u64,
    ) -> Result<Self, DiscordError> {
        let session = davey::DaveSession::new(protocol_version, user_id, channel_id, None)
            .map_err(|error| {
                invalid_data_error(format!("failed to create DAVE session: {error:?}"))
            })?;
        Ok(Self { session })
    }

    pub fn session(&self) -> &davey::DaveSession {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut davey::DaveSession {
        &mut self.session
    }

    pub fn is_ready(&self) -> bool {
        self.session.is_ready()
    }

    pub fn voice_privacy_code(&self) -> Option<&str> {
        self.session.voice_privacy_code()
    }

    pub fn set_external_sender(&mut self, external_sender: &[u8]) -> Result<(), DiscordError> {
        self.session
            .set_external_sender(external_sender)
            .map_err(|error| {
                invalid_data_error(format!("failed to set DAVE external sender: {error:?}"))
            })
    }

    pub fn create_key_package(&mut self) -> Result<Vec<u8>, DiscordError> {
        self.session.create_key_package().map_err(|error| {
            invalid_data_error(format!("failed to create DAVE key package: {error:?}"))
        })
    }

    pub fn process_welcome(&mut self, welcome: &[u8]) -> Result<(), DiscordError> {
        self.session.process_welcome(welcome).map_err(|error| {
            invalid_data_error(format!("failed to process DAVE welcome: {error:?}"))
        })
    }

    pub fn process_commit(&mut self, commit: &[u8]) -> Result<(), DiscordError> {
        self.session.process_commit(commit).map_err(|error| {
            invalid_data_error(format!("failed to process DAVE commit: {error:?}"))
        })
    }

    pub fn process_proposals(
        &mut self,
        operation_type: davey::ProposalsOperationType,
        proposals: &[u8],
        expected_user_ids: Option<&[u64]>,
    ) -> Result<Option<davey::CommitWelcome>, DiscordError> {
        self.session
            .process_proposals(operation_type, proposals, expected_user_ids)
            .map_err(|error| {
                invalid_data_error(format!("failed to process DAVE proposals: {error:?}"))
            })
    }

    pub fn set_passthrough_mode(&mut self, enabled: bool, transition_expiry: Option<u32>) {
        self.session
            .set_passthrough_mode(enabled, transition_expiry);
    }

    pub fn encrypt_opus(&mut self, opus_frame: &[u8]) -> Result<Vec<u8>, DiscordError> {
        self.session
            .encrypt_opus(opus_frame)
            .map(|frame| frame.into_owned())
            .map_err(|error| {
                invalid_data_error(format!("failed to encrypt DAVE Opus frame: {error:?}"))
            })
    }
}

#[cfg(feature = "dave")]
impl VoiceDaveFrameEncryptor for VoiceDaveySession {
    fn encrypt_opus(&mut self, opus_frame: &[u8]) -> Result<Vec<u8>, DiscordError> {
        VoiceDaveySession::encrypt_opus(self, opus_frame)
    }
}

#[cfg(feature = "dave")]
impl VoiceDaveFrameDecryptor for VoiceDaveySession {
    fn decrypt_frame(
        &mut self,
        _rtp: &VoiceRtpHeader,
        user_id: Option<&Snowflake>,
        frame: &VoiceDaveFrame,
    ) -> Result<Vec<u8>, DiscordError> {
        let user_id = user_id
            .and_then(Snowflake::as_u64)
            .ok_or_else(|| invalid_data_error("DAVE frame decrypt requires a mapped user_id"))?;
        self.session
            .decrypt(user_id, davey::MediaType::AUDIO, &frame.bytes)
            .map_err(|error| invalid_data_error(format!("failed to decrypt DAVE frame: {error:?}")))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSpeakingUpdate {
    pub speaking: u64,
    pub ssrc: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Snowflake>,
}

fn parse_rtp_header(bytes: &[u8]) -> Result<VoiceRtpHeader, DiscordError> {
    if bytes.len() < 12 {
        return Err(invalid_data_error(
            "voice RTP packet is shorter than 12 bytes",
        ));
    }

    let version = bytes[0] >> 6;
    let padding = bytes[0] & 0x20 != 0;
    let extension = bytes[0] & 0x10 != 0;
    let csrc_count = usize::from(bytes[0] & 0x0f);
    let marker = bytes[1] & 0x80 != 0;
    let payload_type = bytes[1] & 0x7f;
    let sequence = u16::from_be_bytes([bytes[2], bytes[3]]);
    let timestamp = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let ssrc = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);

    let mut header_len = 12 + csrc_count * 4;
    if bytes.len() < header_len {
        return Err(invalid_data_error(
            "voice RTP packet has truncated CSRC list",
        ));
    }

    if extension {
        if bytes.len() < header_len + 4 {
            return Err(invalid_data_error(
                "voice RTP packet has truncated extension header",
            ));
        }
        let extension_words = usize::from(u16::from_be_bytes([
            bytes[header_len + 2],
            bytes[header_len + 3],
        ]));
        header_len += 4 + extension_words * 4;
        if bytes.len() < header_len {
            return Err(invalid_data_error(
                "voice RTP packet has truncated extension payload",
            ));
        }
    }

    Ok(VoiceRtpHeader {
        version,
        padding,
        extension,
        marker,
        payload_type,
        sequence,
        timestamp,
        ssrc,
        header_len,
    })
}

fn parse_uleb128(bytes: &[u8]) -> Result<(u64, usize), DiscordError> {
    let mut value = 0_u64;
    let mut shift = 0_u32;

    for (index, byte) in bytes.iter().enumerate() {
        let chunk = u64::from(byte & 0x7f);
        if shift >= 64 && chunk != 0 {
            return Err(invalid_data_error("DAVE ULEB128 value overflows u64"));
        }
        value |= chunk
            .checked_shl(shift)
            .ok_or_else(|| invalid_data_error("DAVE ULEB128 value overflows u64"))?;
        if byte & 0x80 == 0 {
            return Ok((value, index + 1));
        }
        shift += 7;
    }

    Err(invalid_data_error("truncated DAVE ULEB128 value"))
}

fn parse_dave_frame(frame: &[u8]) -> Result<VoiceDaveFrame, DiscordError> {
    if frame.len() < 12 {
        return Err(invalid_data_error("DAVE frame is too short"));
    }
    if frame[frame.len() - 2..] != DAVE_MAGIC_MARKER {
        return Err(invalid_data_error("DAVE frame is missing magic marker"));
    }

    let supplemental_size = frame[frame.len() - 3];
    let supplemental_size_usize = usize::from(supplemental_size);
    if supplemental_size_usize < 12 || supplemental_size_usize > frame.len() {
        return Err(invalid_data_error("invalid DAVE supplemental data size"));
    }

    let supplemental_start = frame.len() - supplemental_size_usize;
    let supplemental = &frame[supplemental_start..frame.len() - 3];
    if supplemental.len() < 9 {
        return Err(invalid_data_error("DAVE supplemental data is truncated"));
    }

    let mut auth_tag = [0_u8; 8];
    auth_tag.copy_from_slice(&supplemental[..8]);
    let (nonce, nonce_len) = parse_uleb128(&supplemental[8..])?;
    let nonce =
        u32::try_from(nonce).map_err(|_| invalid_data_error("DAVE frame nonce exceeds 32 bits"))?;

    let mut ranges = Vec::new();
    let mut cursor = 8 + nonce_len;
    while cursor < supplemental.len() {
        let (offset, offset_len) = parse_uleb128(&supplemental[cursor..])?;
        cursor += offset_len;
        if cursor >= supplemental.len() {
            return Err(invalid_data_error(
                "DAVE unencrypted range is missing length",
            ));
        }
        let (len, len_len) = parse_uleb128(&supplemental[cursor..])?;
        cursor += len_len;
        ranges.push(VoiceDaveUnencryptedRange { offset, len });
    }

    Ok(VoiceDaveFrame {
        bytes: frame.to_vec(),
        ciphertext: frame[..supplemental_start].to_vec(),
        auth_tag,
        nonce,
        unencrypted_ranges: ranges,
        supplemental_size,
    })
}

pub struct VoiceRuntimeHandle {
    state_rx: watch::Receiver<VoiceRuntimeState>,
    command_tx: mpsc::UnboundedSender<VoiceGatewayCommand>,
    close_tx: Option<oneshot::Sender<()>>,
    task: JoinHandle<Result<(), DiscordError>>,
    udp_socket: Arc<UdpSocket>,
    outbound_rtp: Arc<Mutex<VoiceOutboundRtpState>>,
}

impl VoiceRuntimeHandle {
    pub fn state(&self) -> VoiceRuntimeState {
        self.state_rx.borrow().clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<VoiceRuntimeState> {
        self.state_rx.clone()
    }

    pub fn udp_socket(&self) -> Arc<UdpSocket> {
        Arc::clone(&self.udp_socket)
    }

    pub async fn recv_raw_udp_packet(
        &self,
        max_len: usize,
    ) -> Result<VoiceRawUdpPacket, DiscordError> {
        if max_len == 0 {
            return Err(invalid_data_error("max_len must be greater than zero"));
        }

        let mut buffer = vec![0_u8; max_len];
        let received = self.udp_socket.recv(&mut buffer).await?;
        buffer.truncate(received);
        Ok(VoiceRawUdpPacket::from_bytes(buffer))
    }

    pub async fn recv_voice_packet(
        &self,
        max_len: usize,
    ) -> Result<VoiceReceivedPacket, DiscordError> {
        let raw = self.recv_raw_udp_packet(max_len).await?;
        let state = self.state();
        let session_description = state
            .session_description
            .as_ref()
            .ok_or_else(|| invalid_data_error("missing voice session description"))?;
        if session_description.dave_protocol_version.unwrap_or(0) > 0 {
            return Err(invalid_data_error(
                "DAVE encrypted voice frames are not supported by recv_voice_packet",
            ));
        }

        let rtp = parse_rtp_header(&raw.bytes)?;
        let opus_frame = decrypt_transport_payload(
            &raw.bytes,
            &rtp,
            &session_description.mode,
            &session_description.secret_key,
        )?;
        let user_id = state.ssrc_users.get(&rtp.ssrc).cloned();

        Ok(VoiceReceivedPacket {
            raw,
            rtp,
            user_id,
            opus_frame,
        })
    }

    pub async fn recv_voice_packet_with_dave<D>(
        &self,
        max_len: usize,
        dave_decryptor: &mut D,
    ) -> Result<VoiceReceivedPacket, DiscordError>
    where
        D: VoiceDaveFrameDecryptor,
    {
        let raw = self.recv_raw_udp_packet(max_len).await?;
        let state = self.state();
        let session_description = state
            .session_description
            .as_ref()
            .ok_or_else(|| invalid_data_error("missing voice session description"))?;
        let rtp = parse_rtp_header(&raw.bytes)?;
        let transport_frame = decrypt_transport_payload(
            &raw.bytes,
            &rtp,
            &session_description.mode,
            &session_description.secret_key,
        )?;
        let user_id = state.ssrc_users.get(&rtp.ssrc).cloned();
        let opus_frame = if session_description.dave_protocol_version.unwrap_or(0) > 0 {
            let dave_frame = parse_dave_frame(&transport_frame)?;
            dave_decryptor.decrypt_frame(&rtp, user_id.as_ref(), &dave_frame)?
        } else {
            transport_frame
        };

        Ok(VoiceReceivedPacket {
            raw,
            rtp,
            user_id,
            opus_frame,
        })
    }

    pub async fn recv_decoded_voice_packet(
        &self,
        decoder: &mut VoiceOpusDecoder,
        max_len: usize,
    ) -> Result<VoiceDecodedPacket, DiscordError> {
        let packet = self.recv_voice_packet(max_len).await?;
        decoder.decode_packet(packet)
    }

    pub async fn recv_decoded_voice_packet_with_dave<D>(
        &self,
        decoder: &mut VoiceOpusDecoder,
        max_len: usize,
        dave_decryptor: &mut D,
    ) -> Result<VoiceDecodedPacket, DiscordError>
    where
        D: VoiceDaveFrameDecryptor,
    {
        let packet = self
            .recv_voice_packet_with_dave(max_len, dave_decryptor)
            .await?;
        decoder.decode_packet(packet)
    }

    pub fn send(&self, command: VoiceGatewayCommand) -> Result<(), DiscordError> {
        self.command_tx
            .send(command)
            .map_err(|error| invalid_data_error(format!("failed to send voice command: {error}")))
    }

    pub fn set_speaking(&self, flags: VoiceSpeakingFlags, delay: u32) -> Result<(), DiscordError> {
        let ssrc = self.state().ready.ssrc;
        self.send(VoiceGatewayCommand::Speaking(
            VoiceSpeakingCommand::new(ssrc).speaking(flags).delay(delay),
        ))
    }

    #[cfg(feature = "dave")]
    pub fn send_dave_transition_ready(&self, transition_id: u64) -> Result<(), DiscordError> {
        self.send(VoiceGatewayCommand::DaveProtocolTransitionReady { transition_id })
    }

    #[cfg(feature = "dave")]
    pub fn send_dave_mls_key_package(
        &self,
        key_package: impl Into<Vec<u8>>,
    ) -> Result<(), DiscordError> {
        self.send(VoiceGatewayCommand::DaveMlsKeyPackage {
            key_package: key_package.into(),
        })
    }

    #[cfg(feature = "dave")]
    pub fn send_dave_mls_commit_welcome(
        &self,
        commit: impl Into<Vec<u8>>,
        welcome: Option<Vec<u8>>,
    ) -> Result<(), DiscordError> {
        self.send(VoiceGatewayCommand::DaveMlsCommitWelcome {
            commit: commit.into(),
            welcome,
        })
    }

    #[cfg(feature = "dave")]
    pub fn send_dave_mls_invalid_commit_welcome(
        &self,
        transition_id: u64,
    ) -> Result<(), DiscordError> {
        self.send(VoiceGatewayCommand::DaveMlsInvalidCommitWelcome { transition_id })
    }

    pub async fn send_opus_frame(
        &self,
        opus_frame: &[u8],
        duration: Duration,
    ) -> Result<VoiceOutboundPacket, DiscordError> {
        self.send_opus_payload(opus_frame, duration, false).await
    }

    async fn send_opus_payload(
        &self,
        opus_payload: &[u8],
        duration: Duration,
        requires_dave: bool,
    ) -> Result<VoiceOutboundPacket, DiscordError> {
        let state = self.state();
        let session_description = state
            .session_description
            .as_ref()
            .ok_or_else(|| invalid_data_error("missing voice session description"))?;
        let dave_enabled = session_description.dave_protocol_version.unwrap_or(0) > 0;
        if requires_dave && !dave_enabled {
            return Err(invalid_data_error(
                "DAVE encrypted Opus send requires an active DAVE session",
            ));
        }
        if !requires_dave && dave_enabled {
            return Err(invalid_data_error(
                "DAVE encrypted voice frames are not supported by send_opus_frame",
            ));
        }

        let packet = {
            let mut outbound_rtp = self.outbound_rtp.lock().await;
            outbound_rtp.build_packet(
                opus_payload,
                duration,
                &session_description.mode,
                &session_description.secret_key,
            )?
        };
        self.udp_socket.send(&packet.bytes).await?;
        Ok(packet)
    }

    #[cfg(feature = "dave")]
    pub async fn send_opus_frame_with_dave<E>(
        &self,
        opus_frame: &[u8],
        duration: Duration,
        encryptor: &mut E,
    ) -> Result<VoiceOutboundPacket, DiscordError>
    where
        E: VoiceDaveFrameEncryptor + ?Sized,
    {
        let encrypted = encryptor.encrypt_opus(opus_frame)?;
        self.send_opus_payload(&encrypted, duration, true).await
    }

    #[cfg(feature = "voice-encode")]
    pub async fn play_audio_source<S>(
        &self,
        source: &mut S,
        encoder: &mut VoiceOpusEncoder,
    ) -> Result<usize, DiscordError>
    where
        S: AudioSource + ?Sized,
    {
        self.play_audio_source_limited(source, encoder, None).await
    }

    #[cfg(feature = "voice-encode")]
    pub async fn play_audio_source_limited<S>(
        &self,
        source: &mut S,
        encoder: &mut VoiceOpusEncoder,
        max_frames: Option<usize>,
    ) -> Result<usize, DiscordError>
    where
        S: AudioSource + ?Sized,
    {
        let frames = encoder.encode_source_frames(source, max_frames)?;
        self.play_opus_frames(frames).await
    }

    pub async fn play_opus_frames<I>(&self, frames: I) -> Result<usize, DiscordError>
    where
        I: IntoIterator<Item = VoiceOpusFrame>,
    {
        self.set_speaking(VoiceSpeakingFlags::MICROPHONE, 0)?;
        let mut sent = 0;
        let mut result = Ok(());
        for frame in frames {
            if let Err(error) = self.send_opus_frame(&frame.bytes, frame.duration).await {
                result = Err(error);
                break;
            }
            sent += 1;
            sleep(frame.duration).await;
        }
        let stop_result = self.set_speaking(VoiceSpeakingFlags::default(), 0);
        result?;
        stop_result?;
        Ok(sent)
    }

    pub async fn close(mut self) -> Result<(), DiscordError> {
        if let Some(close_tx) = self.close_tx.take() {
            let _ = close_tx.send(());
        }

        match self.task.await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) if error.to_string().contains("ResetWithoutClosingHandshake") => Ok(()),
            Ok(result) => result,
            Err(error) => Err(format!("voice runtime task failed: {error}").into()),
        }
    }
}

pub async fn connect(config: VoiceRuntimeConfig) -> Result<VoiceRuntimeHandle, DiscordError> {
    let websocket_url = config.websocket_url();
    let (ws_stream, _) = connect_async(&websocket_url).await?;
    let (mut write, mut read) = ws_stream.split();

    let hello = read_voice_payload(&mut read).await?;
    let heartbeat_interval_ms = read_hello_interval(&hello)?;

    let identify = build_identify_payload(&config);
    write
        .send(WsMessage::Text(identify.to_string().into()))
        .await?;

    let mut last_sequence = hello.get("seq").and_then(Value::as_i64);
    let ready_payload =
        wait_for_voice_opcode(&mut read, VOICE_OP_READY, &mut last_sequence).await?;
    let ready: VoiceGatewayReady = serde_json::from_value(
        ready_payload
            .get("d")
            .cloned()
            .ok_or_else(|| invalid_data_error("missing ready data"))?,
    )?;

    let udp_socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
    udp_socket.connect((&*ready.ip, ready.port)).await?;

    let request = VoiceUdpDiscoveryPacket::request(ready.ssrc);
    udp_socket.send(&request).await?;

    let mut discovery_buffer = [0_u8; VoiceUdpDiscoveryPacket::LEN];
    let received = udp_socket.recv(&mut discovery_buffer).await?;
    let discovery = VoiceUdpDiscoveryPacket::decode(&discovery_buffer[..received])?;
    let selected_mode = select_encryption_mode(&config, &ready)?;

    let select_protocol = VoiceGatewayCommand::SelectProtocol(VoiceSelectProtocolCommand::udp(
        discovery.address.clone(),
        discovery.port,
        selected_mode.clone(),
    ));
    write
        .send(WsMessage::Text(
            select_protocol.payload().to_string().into(),
        ))
        .await?;

    let session_description_payload =
        wait_for_voice_opcode(&mut read, VOICE_OP_SESSION_DESCRIPTION, &mut last_sequence).await?;
    let session_description: VoiceSessionDescription = serde_json::from_value(
        session_description_payload
            .get("d")
            .cloned()
            .ok_or_else(|| invalid_data_error("missing session description data"))?,
    )?;
    let dave_protocol_version = session_description.dave_protocol_version;

    let initial_state = VoiceRuntimeState {
        config,
        heartbeat_interval_ms,
        last_sequence,
        ready,
        discovery,
        selected_mode,
        session_description: Some(session_description),
        ssrc_users: HashMap::new(),
        speaking: HashMap::new(),
        dave: VoiceDaveState {
            protocol_version: dave_protocol_version,
            passthrough: dave_protocol_version.unwrap_or(0) == 0,
            ..VoiceDaveState::default()
        },
        resumed: false,
    };
    let (state_tx, state_rx) = watch::channel(initial_state);
    let (command_tx, mut command_rx) = mpsc::unbounded_channel::<VoiceGatewayCommand>();
    let (close_tx, mut close_rx) = oneshot::channel();
    let udp_socket_handle = Arc::clone(&udp_socket);
    let outbound_rtp = Arc::new(Mutex::new(VoiceOutboundRtpState::new(
        state_tx.borrow().ready.ssrc,
    )));

    let task = tokio::spawn(async move {
        let mut heartbeat = interval(Duration::from_millis(heartbeat_interval_ms));
        let mut heartbeat_nonce = initial_voice_heartbeat_nonce();
        let mut seq_ack = state_tx.borrow().last_sequence;

        loop {
            tokio::select! {
                _ = heartbeat.tick() => {
                    let heartbeat_payload =
                        build_heartbeat_payload(next_voice_heartbeat_nonce(&mut heartbeat_nonce), seq_ack);
                    write.send(WsMessage::Text(heartbeat_payload.to_string().into())).await?;
                }
                command = command_rx.recv() => {
                    match command {
                        Some(command) => {
                            if let Some(bytes) = command.binary_payload() {
                                write.send(WsMessage::Binary(bytes.into())).await?;
                            } else {
                                write.send(WsMessage::Text(command.payload().to_string().into())).await?;
                            }
                        }
                        None => break,
                    }
                }
                _ = &mut close_rx => {
                    let _ = write.send(WsMessage::Close(None)).await;
                    break;
                }
                message = read.next() => {
                    match message {
                        Some(Ok(WsMessage::Text(text))) => {
                            let payload: Value = serde_json::from_str(&text)?;
                            if let Some(seq) = payload.get("seq").and_then(Value::as_i64) {
                                seq_ack = Some(seq);
                                update_state(&state_tx, |state| state.last_sequence = Some(seq))?;
                            }

                            match payload.get("op").and_then(Value::as_u64) {
                                Some(VOICE_OP_SESSION_DESCRIPTION) => {
                                    let description: VoiceSessionDescription = serde_json::from_value(
                                        payload.get("d").cloned().ok_or_else(|| invalid_data_error("missing session description data"))?
                                    )?;
                                    update_state(&state_tx, |state| {
                                        state.dave.protocol_version = description.dave_protocol_version;
                                        state.dave.passthrough = description.dave_protocol_version.unwrap_or(0) == 0;
                                        state.session_description = Some(description);
                                    })?;
                                }
                                Some(VOICE_OP_RESUMED) => {
                                    update_state(&state_tx, |state| state.resumed = true)?;
                                }
                                Some(VOICE_OP_DAVE_PREPARE_TRANSITION) => {
                                    if let Some(data) = payload.get("d") {
                                        update_state(&state_tx, |state| {
                                            state.dave.transition_id = data
                                                .get("transition_id")
                                                .and_then(Value::as_u64);
                                            if data
                                                .get("protocol_version")
                                                .and_then(Value::as_u64)
                                                == Some(0)
                                            {
                                                state.dave.passthrough = true;
                                            }
                                        })?;
                                    }
                                }
                                Some(VOICE_OP_DAVE_EXECUTE_TRANSITION) => {
                                    update_state(&state_tx, |state| {
                                        state.dave.transition_id = None;
                                        state.dave.pending_commit = None;
                                        state.dave.pending_welcome = None;
                                        state.dave.proposals.clear();
                                    })?;
                                }
                                Some(VOICE_OP_DAVE_PREPARE_EPOCH) => {
                                    if let Some(data) = payload.get("d") {
                                        update_state(&state_tx, |state| {
                                            state.dave.transition_id = data
                                                .get("transition_id")
                                                .and_then(Value::as_u64);
                                            state.dave.epoch = data
                                                .get("epoch")
                                                .and_then(Value::as_u64);
                                            if let Some(protocol_version) = data
                                                .get("protocol_version")
                                                .and_then(Value::as_u64)
                                                .and_then(|version| u8::try_from(version).ok())
                                            {
                                                state.dave.protocol_version = Some(protocol_version);
                                                state.dave.passthrough = protocol_version == 0;
                                            }
                                        })?;
                                    }
                                }
                                Some(code) if code == u64::from(VoiceGatewayOpcode::SPEAKING.code()) => {
                                    let update: VoiceSpeakingUpdate = serde_json::from_value(
                                        payload.get("d").cloned().ok_or_else(|| invalid_data_error("missing speaking data"))?
                                    )?;
                                    update_state(&state_tx, |state| {
                                        if let Some(user_id) = update.user_id.clone() {
                                            state.ssrc_users.insert(update.ssrc, user_id);
                                        }
                                        state.speaking.insert(update.ssrc, update);
                                    })?;
                                }
                                Some(VOICE_OP_CLIENTS_CONNECT) => {
                                    if let Some(users) = payload
                                        .get("d")
                                        .and_then(|data| data.get("users"))
                                        .and_then(Value::as_array)
                                    {
                                        let pairs = users
                                            .iter()
                                            .filter_map(|user| {
                                                let user_id = user
                                                    .get("user_id")
                                                    .and_then(|value| serde_json::from_value(value.clone()).ok())?;
                                                let ssrc = user.get("ssrc").and_then(Value::as_u64)? as u32;
                                                Some((ssrc, user_id))
                                            })
                                            .collect::<Vec<_>>();
                                        if !pairs.is_empty() {
                                            update_state(&state_tx, |state| {
                                                for (ssrc, user_id) in pairs {
                                                    state.ssrc_users.insert(ssrc, user_id);
                                                }
                                            })?;
                                        }
                                    }
                                }
                                Some(VOICE_OP_CLIENT_DISCONNECT) => {
                                    if let Some(user_id) = payload
                                        .get("d")
                                        .and_then(|data| data.get("user_id"))
                                        .and_then(|value| serde_json::from_value(value.clone()).ok())
                                    {
                                        update_state(&state_tx, |state| {
                                            state
                                                .ssrc_users
                                                .retain(|_, stored_user_id| stored_user_id != &user_id);
                                        })?;
                                    }
                                }
                                Some(VOICE_OP_HEARTBEAT_ACK) | Some(VOICE_OP_HELLO) | Some(VOICE_OP_READY) | Some(VOICE_OP_HEARTBEAT) | Some(VOICE_OP_RESUME) => {}
                                _ => {}
                            }
                        }
                        Some(Ok(WsMessage::Binary(bytes))) => {
                            if bytes.len() >= 2 {
                                let seq = i64::from(u16::from_be_bytes([bytes[0], bytes[1]]));
                                seq_ack = Some(seq);
                                update_state(&state_tx, |state| state.last_sequence = Some(seq))?;
                            }
                            if bytes.len() >= 3 {
                                let opcode = u64::from(bytes[2]);
                                let payload = bytes[3..].to_vec();
                                match opcode {
                                    VOICE_OP_DAVE_MLS_EXTERNAL_SENDER => {
                                        update_state(&state_tx, |state| {
                                            state.dave.external_sender = Some(payload);
                                        })?;
                                    }
                                    VOICE_OP_DAVE_MLS_PROPOSALS => {
                                        update_state(&state_tx, |state| {
                                            state.dave.proposals.push(payload);
                                        })?;
                                    }
                                    VOICE_OP_DAVE_MLS_ANNOUNCE_COMMIT_TRANSITION => {
                                        update_state(&state_tx, |state| {
                                            state.dave.pending_commit = Some(payload);
                                        })?;
                                    }
                                    VOICE_OP_DAVE_MLS_WELCOME => {
                                        update_state(&state_tx, |state| {
                                            state.dave.pending_welcome = Some(payload);
                                        })?;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Some(Ok(WsMessage::Close(_))) => break,
                        Some(Ok(_)) => {}
                        Some(Err(error)) => return Err(error.into()),
                        None => break,
                    }
                }
            }
        }

        Ok(())
    });

    Ok(VoiceRuntimeHandle {
        state_rx,
        command_tx,
        close_tx: Some(close_tx),
        task,
        udp_socket: udp_socket_handle,
        outbound_rtp,
    })
}

fn build_identify_payload(config: &VoiceRuntimeConfig) -> Value {
    let mut payload = serde_json::json!({
        "op": 0,
        "d": {
            "server_id": config.server_id,
            "user_id": config.user_id,
            "session_id": config.session_id,
            "token": config.token,
        }
    });

    if let Some(max_dave_protocol_version) = config.max_dave_protocol_version {
        payload["d"]["max_dave_protocol_version"] = serde_json::json!(max_dave_protocol_version);
    }

    payload
}

fn build_heartbeat_payload(heartbeat_nonce: u64, seq_ack: Option<i64>) -> Value {
    serde_json::json!({
        "op": VOICE_OP_HEARTBEAT,
        "d": {
            "t": heartbeat_nonce,
            "seq_ack": seq_ack.unwrap_or(-1),
        }
    })
}

fn initial_voice_heartbeat_nonce() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0)
}

fn next_voice_heartbeat_nonce(current: &mut u64) -> u64 {
    *current = current.wrapping_add(1);
    *current
}

fn decrypt_transport_payload(
    packet: &[u8],
    rtp: &VoiceRtpHeader,
    mode: &VoiceEncryptionMode,
    secret_key: &[u8],
) -> Result<Vec<u8>, DiscordError> {
    if secret_key.len() != 32 {
        return Err(invalid_data_error("voice secret_key must be 32 bytes"));
    }
    if packet.len() < rtp.header_len + 4 {
        return Err(invalid_data_error(
            "voice RTP packet is missing the RTP-size nonce suffix",
        ));
    }

    let nonce_suffix_offset = packet.len() - 4;
    let nonce_suffix = &packet[nonce_suffix_offset..];
    let aad = &packet[..rtp.header_len];
    let mut encrypted = packet[rtp.header_len..nonce_suffix_offset].to_vec();

    if mode == &VoiceEncryptionMode::aead_aes256_gcm_rtpsize() {
        let cipher = Aes256Gcm::new_from_slice(secret_key)
            .map_err(|_| invalid_data_error("invalid AES-GCM voice secret key"))?;
        let mut nonce = [0_u8; 12];
        nonce[8..12].copy_from_slice(nonce_suffix);
        cipher
            .decrypt_in_place(AesNonce::from_slice(&nonce), aad, &mut encrypted)
            .map_err(|_| invalid_data_error("failed to decrypt AES-GCM voice packet"))?;
        return Ok(encrypted);
    }

    if mode == &VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize() {
        let cipher = XChaCha20Poly1305::new_from_slice(secret_key)
            .map_err(|_| invalid_data_error("invalid XChaCha20-Poly1305 voice secret key"))?;
        let mut nonce = [0_u8; 24];
        nonce[20..24].copy_from_slice(nonce_suffix);
        cipher
            .decrypt_in_place(XNonce::from_slice(&nonce), aad, &mut encrypted)
            .map_err(|_| invalid_data_error("failed to decrypt XChaCha20-Poly1305 voice packet"))?;
        return Ok(encrypted);
    }

    Err(invalid_data_error(format!(
        "unsupported voice encryption mode for receive: {mode:?}"
    )))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VoiceOutboundEncryptParams {
    sequence: u16,
    timestamp: u32,
    ssrc: u32,
    payload_type: u8,
    nonce_suffix: [u8; 4],
}

fn encrypt_transport_payload(
    params: VoiceOutboundEncryptParams,
    opus_frame: &[u8],
    mode: &VoiceEncryptionMode,
    secret_key: &[u8],
) -> Result<Vec<u8>, DiscordError> {
    if secret_key.len() != 32 {
        return Err(invalid_data_error("voice secret_key must be 32 bytes"));
    }

    let mut packet = build_rtp_header(
        params.sequence,
        params.timestamp,
        params.ssrc,
        params.payload_type,
    );
    let aad = packet.clone();
    let mut encrypted = opus_frame.to_vec();

    if mode == &VoiceEncryptionMode::aead_aes256_gcm_rtpsize() {
        let cipher = Aes256Gcm::new_from_slice(secret_key)
            .map_err(|_| invalid_data_error("invalid AES-GCM voice secret key"))?;
        let mut nonce = [0_u8; 12];
        nonce[8..12].copy_from_slice(&params.nonce_suffix);
        cipher
            .encrypt_in_place(AesNonce::from_slice(&nonce), &aad, &mut encrypted)
            .map_err(|_| invalid_data_error("failed to encrypt AES-GCM voice packet"))?;
    } else if mode == &VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize() {
        let cipher = XChaCha20Poly1305::new_from_slice(secret_key)
            .map_err(|_| invalid_data_error("invalid XChaCha20-Poly1305 voice secret key"))?;
        let mut nonce = [0_u8; 24];
        nonce[20..24].copy_from_slice(&params.nonce_suffix);
        cipher
            .encrypt_in_place(XNonce::from_slice(&nonce), &aad, &mut encrypted)
            .map_err(|_| invalid_data_error("failed to encrypt XChaCha20-Poly1305 voice packet"))?;
    } else {
        return Err(invalid_data_error(format!(
            "unsupported voice encryption mode for send: {mode:?}"
        )));
    }

    packet.extend_from_slice(&encrypted);
    packet.extend_from_slice(&params.nonce_suffix);
    Ok(packet)
}

fn build_rtp_header(sequence: u16, timestamp: u32, ssrc: u32, payload_type: u8) -> Vec<u8> {
    let mut packet = vec![RTP_VERSION << 6, payload_type & 0x7f];
    packet.extend_from_slice(&sequence.to_be_bytes());
    packet.extend_from_slice(&timestamp.to_be_bytes());
    packet.extend_from_slice(&ssrc.to_be_bytes());
    packet
}

fn timestamp_increment(sample_rate: u32, duration: Duration) -> u32 {
    let nanos = duration.as_nanos();
    let samples = (u128::from(sample_rate) * nanos) / 1_000_000_000;
    samples.max(1).min(u128::from(u32::MAX)) as u32
}

fn read_hello_interval(payload: &Value) -> Result<u64, DiscordError> {
    payload
        .get("d")
        .and_then(|data| data.get("heartbeat_interval"))
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid_data_error("missing voice hello heartbeat interval"))
}

fn select_encryption_mode(
    config: &VoiceRuntimeConfig,
    ready: &VoiceGatewayReady,
) -> Result<VoiceEncryptionMode, DiscordError> {
    if let Some(preferred_mode) = &config.preferred_mode {
        if ready.modes.contains(preferred_mode) {
            return Ok(preferred_mode.clone());
        }
    }

    ready
        .modes
        .first()
        .cloned()
        .ok_or_else(|| invalid_data_error("voice ready payload did not include encryption modes"))
}

fn update_state(
    state_tx: &watch::Sender<VoiceRuntimeState>,
    update: impl FnOnce(&mut VoiceRuntimeState),
) -> Result<(), DiscordError> {
    let mut state = state_tx.borrow().clone();
    update(&mut state);
    state_tx.send(state).map_err(|error| {
        invalid_data_error(format!("failed to publish voice runtime state: {error}"))
    })
}

async fn read_voice_payload(
    read: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
) -> Result<Value, DiscordError> {
    loop {
        match read.next().await {
            Some(Ok(WsMessage::Text(text))) => return Ok(serde_json::from_str(&text)?),
            Some(Ok(_)) => {}
            Some(Err(error)) => return Err(error.into()),
            None => return Err(invalid_data_error("voice websocket closed unexpectedly")),
        }
    }
}

async fn wait_for_voice_opcode(
    read: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    opcode: u64,
    last_sequence: &mut Option<i64>,
) -> Result<Value, DiscordError> {
    loop {
        let payload = read_voice_payload(read).await?;
        if let Some(seq) = payload.get("seq").and_then(Value::as_i64) {
            *last_sequence = Some(seq);
        }
        if payload.get("op").and_then(Value::as_u64) == Some(opcode) {
            return Ok(payload);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use aes_gcm::aead::{AeadInPlace, KeyInit};
    use aes_gcm::{Aes256Gcm, Nonce as AesNonce};
    use chacha20poly1305::{XChaCha20Poly1305, XNonce};
    use futures_util::{SinkExt, StreamExt};
    use serde_json::Value;
    use tokio::net::{TcpListener, UdpSocket};
    use tokio::sync::{oneshot, watch};
    use tokio::time::{timeout, Duration};
    use tokio_tungstenite::{accept_async, tungstenite::Message as WsMessage};

    use super::{
        build_heartbeat_payload, build_identify_payload, connect, decrypt_transport_payload,
        next_voice_heartbeat_nonce, parse_dave_frame, parse_rtp_header, parse_uleb128,
        read_hello_interval, select_encryption_mode, update_state, VoiceDaveState,
        VoiceOpusDecoder, VoiceOutboundRtpState, VoiceRuntimeConfig, VoiceRuntimeState,
        VoiceSessionDescription,
    };
    #[cfg(feature = "voice-encode")]
    use super::{AudioMixer, AudioSource, PcmFrame, VoiceOpusEncoder};
    use crate::voice::{
        VoiceEncryptionMode, VoiceGatewayCommand, VoiceGatewayReady, VoiceSpeakingFlags,
        VoiceUdpDiscoveryPacket,
    };

    fn encrypt_aes_rtp_packet(
        secret_key: &[u8; 32],
        sequence: u16,
        timestamp: u32,
        ssrc: u32,
        nonce_suffix: [u8; 4],
        opus_frame: &[u8],
    ) -> Vec<u8> {
        let mut packet = vec![0x80, 0x78];
        packet.extend_from_slice(&sequence.to_be_bytes());
        packet.extend_from_slice(&timestamp.to_be_bytes());
        packet.extend_from_slice(&ssrc.to_be_bytes());

        let cipher = Aes256Gcm::new_from_slice(secret_key).unwrap();
        let mut nonce = [0_u8; 12];
        nonce[8..12].copy_from_slice(&nonce_suffix);
        let mut encrypted = opus_frame.to_vec();
        cipher
            .encrypt_in_place(AesNonce::from_slice(&nonce), &packet, &mut encrypted)
            .unwrap();
        packet.extend_from_slice(&encrypted);
        packet.extend_from_slice(&nonce_suffix);
        packet
    }

    fn encrypt_xchacha_rtp_packet(
        secret_key: &[u8; 32],
        sequence: u16,
        timestamp: u32,
        ssrc: u32,
        nonce_suffix: [u8; 4],
        opus_frame: &[u8],
    ) -> Vec<u8> {
        let mut packet = vec![0x80, 0x78];
        packet.extend_from_slice(&sequence.to_be_bytes());
        packet.extend_from_slice(&timestamp.to_be_bytes());
        packet.extend_from_slice(&ssrc.to_be_bytes());

        let cipher = XChaCha20Poly1305::new_from_slice(secret_key).unwrap();
        let mut nonce = [0_u8; 24];
        nonce[20..24].copy_from_slice(&nonce_suffix);
        let mut encrypted = opus_frame.to_vec();
        cipher
            .encrypt_in_place(XNonce::from_slice(&nonce), &packet, &mut encrypted)
            .unwrap();
        packet.extend_from_slice(&encrypted);
        packet.extend_from_slice(&nonce_suffix);
        packet
    }

    #[test]
    fn voice_runtime_config_normalizes_voice_gateway_url() {
        let config =
            VoiceRuntimeConfig::new("1", "2", "session", "token", "voice.discord.media:443");
        assert_eq!(config.websocket_url(), "wss://voice.discord.media:443/?v=8");
    }

    #[test]
    fn voice_runtime_config_clamps_version_and_extends_existing_query() {
        let config = VoiceRuntimeConfig::new(
            "1",
            "2",
            "session",
            "token",
            "ws://127.0.0.1/socket?encoding=json",
        )
        .gateway_version(2)
        .preferred_mode(VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize())
        .max_dave_protocol_version(3);

        assert_eq!(
            config.websocket_url(),
            "ws://127.0.0.1/socket?encoding=json&v=4"
        );
        assert_eq!(
            config.preferred_mode,
            Some(VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize())
        );
        assert_eq!(config.max_dave_protocol_version, Some(3));
    }

    #[test]
    fn voice_runtime_helper_builders_cover_optional_and_fallback_fields() {
        let config = VoiceRuntimeConfig {
            max_dave_protocol_version: None,
            ..VoiceRuntimeConfig::new("1", "2", "session", "token", "voice.discord.media")
        };
        assert_eq!(
            build_identify_payload(&config),
            serde_json::json!({
                "op": 0,
                "d": {
                    "server_id": "1",
                    "user_id": "2",
                    "session_id": "session",
                    "token": "token",
                }
            })
        );
        assert_eq!(
            build_heartbeat_payload(55, None),
            serde_json::json!({
                "op": 3,
                "d": {
                    "t": 55,
                    "seq_ack": -1,
                }
            })
        );
        assert_eq!(
            build_heartbeat_payload(55, Some(7)),
            serde_json::json!({
                "op": 3,
                "d": {
                    "t": 55,
                    "seq_ack": 7,
                }
            })
        );
    }

    #[test]
    fn voice_runtime_heartbeat_nonce_does_not_reuse_interval_value() {
        let heartbeat_interval_ms = 5_000;
        let mut nonce = heartbeat_interval_ms;

        let first = next_voice_heartbeat_nonce(&mut nonce);
        let second = next_voice_heartbeat_nonce(&mut nonce);

        assert_ne!(first, heartbeat_interval_ms);
        assert_ne!(first, second);
        assert_eq!(
            build_heartbeat_payload(first, None)["d"]["t"],
            serde_json::json!(first)
        );
    }

    #[test]
    fn voice_runtime_read_hello_interval_and_mode_selection_handle_edge_cases() {
        assert_eq!(
            read_hello_interval(&serde_json::json!({
                "d": { "heartbeat_interval": 250 }
            }))
            .unwrap(),
            250
        );
        assert!(read_hello_interval(&serde_json::json!({ "d": {} }))
            .unwrap_err()
            .to_string()
            .contains("missing voice hello heartbeat interval"));

        let preferred = VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize();
        let fallback = VoiceEncryptionMode::aead_aes256_gcm_rtpsize();
        let ready = VoiceGatewayReady::new(42, "127.0.0.1", 5000)
            .mode(fallback.clone())
            .mode(preferred.clone());
        let config = VoiceRuntimeConfig::new("1", "2", "session", "token", "voice.discord.media")
            .preferred_mode(preferred.clone());
        assert_eq!(select_encryption_mode(&config, &ready).unwrap(), preferred);

        let fallback_only = VoiceGatewayReady::new(42, "127.0.0.1", 5000).mode(fallback.clone());
        let config_without_match =
            VoiceRuntimeConfig::new("1", "2", "session", "token", "voice.discord.media")
                .preferred_mode(VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize());
        assert_eq!(
            select_encryption_mode(&config_without_match, &fallback_only).unwrap(),
            fallback
        );

        let empty_ready = VoiceGatewayReady::new(42, "127.0.0.1", 5000);
        assert!(select_encryption_mode(
            &VoiceRuntimeConfig::new("1", "2", "session", "token", "voice.discord.media"),
            &empty_ready
        )
        .unwrap_err()
        .to_string()
        .contains("did not include encryption modes"));
    }

    #[test]
    fn voice_receive_decrypts_aes_gcm_rtp_size_packets() {
        let secret_key = [7_u8; 32];
        let opus_frame = [0xf8, 0xff, 0xfe];
        let packet =
            encrypt_aes_rtp_packet(&secret_key, 0x1234, 48_000, 42, [0, 0, 0, 9], &opus_frame);

        let rtp = parse_rtp_header(&packet).unwrap();
        assert_eq!(rtp.version, 2);
        assert_eq!(rtp.payload_type, 120);
        assert_eq!(rtp.sequence, 0x1234);
        assert_eq!(rtp.timestamp, 48_000);
        assert_eq!(rtp.ssrc, 42);
        assert_eq!(rtp.header_len, 12);

        let decrypted = decrypt_transport_payload(
            &packet,
            &rtp,
            &VoiceEncryptionMode::aead_aes256_gcm_rtpsize(),
            &secret_key,
        )
        .unwrap();
        assert_eq!(decrypted, opus_frame);
    }

    #[test]
    fn voice_receive_decrypts_xchacha_rtp_size_packets() {
        let secret_key = [9_u8; 32];
        let opus_frame = [0x11, 0x22, 0x33, 0x44];
        let packet =
            encrypt_xchacha_rtp_packet(&secret_key, 0x2233, 96_000, 99, [0, 0, 0, 10], &opus_frame);

        let rtp = parse_rtp_header(&packet).unwrap();
        let decrypted = decrypt_transport_payload(
            &packet,
            &rtp,
            &VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize(),
            &secret_key,
        )
        .unwrap();
        assert_eq!(rtp.ssrc, 99);
        assert_eq!(decrypted, opus_frame);
    }

    #[test]
    fn voice_outbound_rtp_state_encrypts_and_advances_counters() {
        let secret_key = [7_u8; 32];
        let opus_frame = [0xf8, 0xff, 0xfe];
        let mut state = VoiceOutboundRtpState::with_counters(42, 0x1234, 48_000, 9);

        let packet = state
            .build_packet(
                &opus_frame,
                Duration::from_millis(20),
                &VoiceEncryptionMode::aead_aes256_gcm_rtpsize(),
                &secret_key,
            )
            .unwrap();

        assert_eq!(packet.rtp.version, 2);
        assert_eq!(packet.rtp.payload_type, 120);
        assert_eq!(packet.rtp.sequence, 0x1234);
        assert_eq!(packet.rtp.timestamp, 48_000);
        assert_eq!(packet.rtp.ssrc, 42);
        assert_eq!(packet.nonce_suffix, [0, 0, 0, 9]);
        assert_eq!(state.sequence, 0x1235);
        assert_eq!(state.timestamp, 48_960);
        assert_eq!(state.nonce_suffix, 10);

        let decrypted = decrypt_transport_payload(
            &packet.bytes,
            &packet.rtp,
            &VoiceEncryptionMode::aead_aes256_gcm_rtpsize(),
            &secret_key,
        )
        .unwrap();
        assert_eq!(decrypted, opus_frame);
    }

    #[test]
    fn voice_outbound_rtp_state_supports_xchacha_and_rejects_empty_frames() {
        let secret_key = [9_u8; 32];
        let opus_frame = [0x11, 0x22, 0x33, 0x44];
        let mut state = VoiceOutboundRtpState::with_counters(99, 0x2233, 96_000, 10);

        let packet = state
            .build_packet(
                &opus_frame,
                Duration::from_millis(40),
                &VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize(),
                &secret_key,
            )
            .unwrap();
        let decrypted = decrypt_transport_payload(
            &packet.bytes,
            &packet.rtp,
            &VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize(),
            &secret_key,
        )
        .unwrap();
        assert_eq!(decrypted, opus_frame);
        assert_eq!(state.timestamp, 97_920);

        assert!(state
            .build_packet(
                &[],
                Duration::from_millis(20),
                &VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize(),
                &secret_key,
            )
            .unwrap_err()
            .to_string()
            .contains("opus frame must not be empty"));
    }

    #[test]
    fn voice_opus_decoder_decodes_discord_silence_frame() {
        let mut decoder = VoiceOpusDecoder::discord_default().unwrap();
        let (samples_per_channel, pcm) = decoder
            .decode_opus_frame(&[0xf8, 0xff, 0xfe], false)
            .unwrap();

        assert_eq!(decoder.sample_rate(), 48_000);
        assert_eq!(decoder.channels(), 2);
        assert!(samples_per_channel > 0);
        assert_eq!(pcm.len(), samples_per_channel * 2);
    }

    #[cfg(feature = "voice-encode")]
    #[derive(Clone)]
    struct OneFrameSource {
        frame: Option<PcmFrame>,
    }

    #[cfg(feature = "voice-encode")]
    impl AudioSource for OneFrameSource {
        fn next_pcm_frame(&mut self) -> Result<Option<PcmFrame>, crate::error::DiscordError> {
            Ok(self.frame.take())
        }
    }

    #[cfg(feature = "voice-encode")]
    #[test]
    fn pcm_frame_accepts_only_discord_stereo_20ms_shape() {
        let frame = PcmFrame::discord_stereo_20ms(vec![0.0; 1_920]).unwrap();

        assert_eq!(frame.sample_rate(), 48_000);
        assert_eq!(frame.channels(), 2);
        assert_eq!(frame.samples_per_channel(), 960);
        assert_eq!(frame.duration(), Duration::from_millis(20));

        assert!(PcmFrame::discord_stereo_20ms(vec![0.0; 1_919])
            .unwrap_err()
            .to_string()
            .contains("1920"));
    }

    #[cfg(feature = "voice-encode")]
    #[test]
    fn audio_mixer_sums_sources_clamps_and_removes_finished_sources() {
        let mut mixer = AudioMixer::new();
        mixer.push_source(OneFrameSource {
            frame: Some(PcmFrame::discord_stereo_20ms(vec![0.75; 1_920]).unwrap()),
        });
        mixer.push_source(OneFrameSource {
            frame: Some(PcmFrame::discord_stereo_20ms(vec![0.5; 1_920]).unwrap()),
        });

        let mixed = mixer.next_pcm_frame().unwrap().unwrap();
        assert_eq!(mixed.samples()[0], 1.0);
        assert_eq!(mixed.samples()[1], 1.0);
        assert_eq!(mixer.active_sources(), 2);

        assert!(mixer.next_pcm_frame().unwrap().is_none());
        assert_eq!(mixer.active_sources(), 0);
    }

    #[cfg(feature = "voice-encode")]
    #[test]
    fn voice_opus_encoder_encodes_discord_pcm_with_audio_cbr_settings() {
        let mut encoder = VoiceOpusEncoder::discord_music()
            .unwrap()
            .with_bitrate_bps(96_000)
            .with_cbr(true);
        let frame = PcmFrame::discord_stereo_20ms(vec![0.0; 1_920]).unwrap();

        assert_eq!(encoder.sample_rate(), 48_000);
        assert_eq!(encoder.channels(), 2);
        assert_eq!(encoder.samples_per_channel(), 960);
        assert_eq!(encoder.bitrate_bps(), 96_000);
        assert!(encoder.use_cbr());

        let encoded = encoder.encode_pcm_frame(&frame).unwrap();
        assert!(!encoded.bytes.is_empty());
        assert_eq!(encoded.duration, Duration::from_millis(20));
    }

    #[test]
    fn voice_dave_frame_parser_reads_trailer_and_ranges() {
        assert_eq!(parse_uleb128(&[0xac, 0x02]).unwrap(), (300, 2));

        let mut frame = vec![1, 2, 3];
        frame.extend_from_slice(&[9_u8; 8]);
        frame.extend_from_slice(&[0xac, 0x02]);
        frame.extend_from_slice(&[5, 6]);
        frame.push(15);
        frame.extend_from_slice(&[0xfa, 0xfa]);

        let parsed = parse_dave_frame(&frame).unwrap();
        assert_eq!(parsed.ciphertext, vec![1, 2, 3]);
        assert_eq!(parsed.auth_tag, [9_u8; 8]);
        assert_eq!(parsed.nonce, 300);
        assert_eq!(parsed.supplemental_size, 15);
        assert_eq!(
            parsed.unencrypted_ranges,
            vec![super::VoiceDaveUnencryptedRange { offset: 5, len: 6 }]
        );
    }

    #[cfg(feature = "dave")]
    #[test]
    fn voice_davey_decryptor_wraps_session_lifecycle_entrypoints() {
        let mut decryptor = super::VoiceDaveyDecryptor::new(
            std::num::NonZeroU16::new(davey::DAVE_PROTOCOL_VERSION).unwrap(),
            42,
            100,
        )
        .unwrap();

        assert!(!decryptor.is_ready());
        assert_eq!(decryptor.session().user_id(), 42);
        assert_eq!(decryptor.session().channel_id(), 100);
        assert!(decryptor.encrypt_opus(&[0xf8, 0xff, 0xfe]).is_err());
        decryptor.set_passthrough_mode(true, Some(1));
    }

    #[test]
    fn voice_runtime_update_state_publishes_changes_and_reports_closed_receivers() {
        let initial_state = VoiceRuntimeState {
            config: VoiceRuntimeConfig::new("1", "2", "session", "token", "voice.discord.media"),
            heartbeat_interval_ms: 250,
            last_sequence: Some(3),
            ready: VoiceGatewayReady::new(42, "127.0.0.1", 5000),
            discovery: VoiceUdpDiscoveryPacket {
                ssrc: 42,
                address: "203.0.113.7".to_string(),
                port: 5000,
            },
            selected_mode: VoiceEncryptionMode::aead_aes256_gcm_rtpsize(),
            session_description: None,
            ssrc_users: HashMap::new(),
            speaking: HashMap::new(),
            dave: VoiceDaveState::default(),
            resumed: false,
        };
        let (state_tx, state_rx) = watch::channel(initial_state.clone());
        update_state(&state_tx, |state| {
            state.last_sequence = Some(9);
            state.resumed = true;
        })
        .unwrap();

        let updated = state_rx.borrow().clone();
        assert_eq!(updated.last_sequence, Some(9));
        assert!(updated.resumed);

        let (closed_tx, closed_rx) = watch::channel(initial_state);
        drop(closed_rx);
        assert!(update_state(&closed_tx, |state| state.resumed = true)
            .unwrap_err()
            .to_string()
            .contains("failed to publish voice runtime state"));
    }

    #[tokio::test]
    async fn voice_runtime_connects_and_completes_handshake() {
        let udp_listener = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let udp_port = udp_listener.local_addr().unwrap().port();

        let tcp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let ws_port = tcp_listener.local_addr().unwrap().port();
        let (speaking_tx, speaking_rx) = oneshot::channel();

        let server = tokio::spawn(async move {
            let mut speaking_tx = Some(speaking_tx);
            let (tcp_stream, _) = tcp_listener.accept().await.unwrap();
            let mut ws = accept_async(tcp_stream).await.unwrap();

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 8,
                    "d": { "heartbeat_interval": 20 }
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            let identify = ws.next().await.unwrap().unwrap().into_text().unwrap();
            let identify_payload: Value = serde_json::from_str(&identify).unwrap();
            assert_eq!(identify_payload["op"], serde_json::json!(0));

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 2,
                    "d": {
                        "ssrc": 42,
                        "ip": "127.0.0.1",
                        "port": udp_port,
                        "modes": ["aead_aes256_gcm_rtpsize"]
                    },
                    "seq": 7
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            let mut discovery_buffer = [0_u8; VoiceUdpDiscoveryPacket::LEN];
            let (received, remote_addr) =
                udp_listener.recv_from(&mut discovery_buffer).await.unwrap();
            assert_eq!(received, VoiceUdpDiscoveryPacket::LEN);

            let mut response = VoiceUdpDiscoveryPacket::request(42);
            response[..2].copy_from_slice(&VoiceUdpDiscoveryPacket::RESPONSE_TYPE.to_be_bytes());
            response[2..4].copy_from_slice(&VoiceUdpDiscoveryPacket::BODY_LEN.to_be_bytes());
            let address = b"203.0.113.7";
            response[8..8 + address.len()].copy_from_slice(address);
            response[72..74].copy_from_slice(&remote_addr.port().to_be_bytes());
            udp_listener.send_to(&response, remote_addr).await.unwrap();

            let select_protocol = ws.next().await.unwrap().unwrap().into_text().unwrap();
            let select_protocol_payload: Value = serde_json::from_str(&select_protocol).unwrap();
            assert_eq!(select_protocol_payload["op"], serde_json::json!(1));

            let secret_key = vec![7_u8; 32];
            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 4,
                    "d": {
                        "mode": "aead_aes256_gcm_rtpsize",
                        "secret_key": secret_key,
                        "dave_protocol_version": 0
                    }
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            let raw_udp_packet = encrypt_aes_rtp_packet(
                &[7_u8; 32],
                0x1234,
                42,
                42,
                [0, 0, 0, 1],
                &[0xf8, 0xff, 0xfe],
            );
            udp_listener
                .send_to(&raw_udp_packet, remote_addr)
                .await
                .unwrap();
            udp_listener
                .send_to(&raw_udp_packet, remote_addr)
                .await
                .unwrap();
            udp_listener
                .send_to(&raw_udp_packet, remote_addr)
                .await
                .unwrap();

            loop {
                let message = ws.next().await.unwrap().unwrap();
                match message {
                    WsMessage::Text(text) => {
                        let payload: Value = serde_json::from_str(&text).unwrap();
                        if payload["op"] == serde_json::json!(5) {
                            if let Some(speaking_tx) = speaking_tx.take() {
                                let _ = speaking_tx.send(());
                            }
                        }
                    }
                    WsMessage::Close(frame) => {
                        let _ = ws.send(WsMessage::Close(frame)).await;
                        break;
                    }
                    _ => {}
                }
            }
        });

        let handle = connect(VoiceRuntimeConfig::new(
            "1",
            "2",
            "session",
            "token",
            format!("ws://127.0.0.1:{ws_port}"),
        ))
        .await
        .unwrap();

        assert_eq!(handle.state().ready.ssrc, 42);
        assert_eq!(handle.state().discovery.address, "203.0.113.7");
        let raw_packet = handle.recv_raw_udp_packet(64).await.unwrap();
        assert_eq!(raw_packet.version, Some(2));
        assert_eq!(raw_packet.payload_type, Some(120));
        assert_eq!(raw_packet.sequence, Some(0x1234));
        assert_eq!(raw_packet.timestamp, Some(42));
        assert_eq!(raw_packet.ssrc, Some(42));
        assert!(raw_packet.bytes.len() > 16);
        let received = handle.recv_voice_packet(64).await.unwrap();
        assert_eq!(received.rtp.ssrc, 42);
        assert_eq!(received.opus_frame, vec![0xf8, 0xff, 0xfe]);
        let mut decoder = VoiceOpusDecoder::discord_default().unwrap();
        let decoded = handle
            .recv_decoded_voice_packet(&mut decoder, 64)
            .await
            .unwrap();
        assert_eq!(decoded.packet.rtp.ssrc, 42);
        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.pcm.len(), decoded.samples_per_channel * 2);
        handle
            .set_speaking(VoiceSpeakingFlags::MICROPHONE, 0)
            .unwrap();
        speaking_rx.await.unwrap();
        handle.close().await.unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn voice_runtime_loop_updates_state_and_sends_custom_commands() {
        let udp_listener = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let udp_port = udp_listener.local_addr().unwrap().port();

        let tcp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let ws_port = tcp_listener.local_addr().unwrap().port();
        let (heartbeat_tx, heartbeat_rx) = oneshot::channel();
        let (dave_commands_tx, dave_commands_rx) = oneshot::channel();

        let server = tokio::spawn(async move {
            let mut heartbeat_tx = Some(heartbeat_tx);
            let mut dave_commands_tx = Some(dave_commands_tx);
            let mut saw_transition_ready = false;
            let mut saw_key_package = false;
            let mut saw_commit_welcome = false;
            let mut saw_invalid_commit_welcome = false;
            let (tcp_stream, _) = tcp_listener.accept().await.unwrap();
            let mut ws = accept_async(tcp_stream).await.unwrap();

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 8,
                    "d": { "heartbeat_interval": 5_000 },
                    "seq": 3
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            let identify = ws.next().await.unwrap().unwrap().into_text().unwrap();
            let identify_payload: Value = serde_json::from_str(&identify).unwrap();
            assert_eq!(identify_payload["d"]["max_dave_protocol_version"], 1);

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 2,
                    "d": {
                        "ssrc": 42,
                        "ip": "127.0.0.1",
                        "port": udp_port,
                        "modes": [
                            "aead_aes256_gcm_rtpsize",
                            "aead_xchacha20_poly1305_rtpsize"
                        ]
                    },
                    "seq": 7
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            let mut discovery_buffer = [0_u8; VoiceUdpDiscoveryPacket::LEN];
            let (received, remote_addr) =
                udp_listener.recv_from(&mut discovery_buffer).await.unwrap();
            assert_eq!(received, VoiceUdpDiscoveryPacket::LEN);

            let mut response = VoiceUdpDiscoveryPacket::request(42);
            response[..2].copy_from_slice(&VoiceUdpDiscoveryPacket::RESPONSE_TYPE.to_be_bytes());
            response[2..4].copy_from_slice(&VoiceUdpDiscoveryPacket::BODY_LEN.to_be_bytes());
            let address = b"203.0.113.7";
            response[8..8 + address.len()].copy_from_slice(address);
            response[72..74].copy_from_slice(&remote_addr.port().to_be_bytes());
            udp_listener.send_to(&response, remote_addr).await.unwrap();

            let select_protocol = ws.next().await.unwrap().unwrap().into_text().unwrap();
            let select_protocol_payload: Value = serde_json::from_str(&select_protocol).unwrap();
            assert_eq!(
                select_protocol_payload["d"]["data"]["mode"],
                serde_json::json!("aead_xchacha20_poly1305_rtpsize")
            );

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 4,
                    "d": {
                        "mode": "aead_xchacha20_poly1305_rtpsize",
                        "secret_key": [1, 2, 3, 4]
                    },
                    "seq": 9
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 4,
                    "d": {
                        "mode": "aead_xchacha20_poly1305_rtpsize",
                        "secret_key": [9, 9],
                        "audio_codec": "opus"
                    },
                    "seq": 11
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 9,
                    "seq": 12
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 24,
                    "d": {
                        "transition_id": 99,
                        "epoch": 2,
                        "protocol_version": 1
                    },
                    "seq": 13
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
            ws.send(WsMessage::Binary(vec![0, 14, 25, 1, 2, 3].into()))
                .await
                .unwrap();
            ws.send(WsMessage::Binary(vec![0, 15, 27, 4, 5].into()))
                .await
                .unwrap();
            ws.send(WsMessage::Binary(vec![0, 16, 29, 6].into()))
                .await
                .unwrap();
            ws.send(WsMessage::Binary(vec![0, 17, 30, 7].into()))
                .await
                .unwrap();
            ws.send(WsMessage::Binary(vec![0, 25].into()))
                .await
                .unwrap();

            loop {
                let message = ws.next().await.unwrap().unwrap();
                match message {
                    WsMessage::Text(text) => {
                        let payload: Value = serde_json::from_str(&text).unwrap();
                        if payload["op"] == serde_json::json!(3)
                            && payload["d"] == serde_json::json!(55)
                        {
                            if let Some(heartbeat_tx) = heartbeat_tx.take() {
                                let _ = heartbeat_tx.send(());
                            }
                        }
                        if payload["op"] == serde_json::json!(23)
                            && payload["d"]["transition_id"] == serde_json::json!(99)
                        {
                            saw_transition_ready = true;
                        }
                        if payload["op"] == serde_json::json!(31)
                            && payload["d"]["transition_id"] == serde_json::json!(99)
                        {
                            saw_invalid_commit_welcome = true;
                        }
                    }
                    WsMessage::Binary(bytes) => {
                        if bytes.as_ref() == [26, 1, 2, 3] {
                            saw_key_package = true;
                        }
                        if bytes.as_ref() == [28, 4, 5, 6, 7] {
                            saw_commit_welcome = true;
                        }
                    }
                    WsMessage::Close(frame) => {
                        let _ = ws.send(WsMessage::Close(frame)).await;
                        break;
                    }
                    _ => {}
                }
                if saw_transition_ready
                    && saw_key_package
                    && saw_commit_welcome
                    && saw_invalid_commit_welcome
                {
                    if let Some(dave_commands_tx) = dave_commands_tx.take() {
                        let _ = dave_commands_tx.send(());
                    }
                }
            }
        });

        let handle = connect(
            VoiceRuntimeConfig::new(
                "1",
                "2",
                "session",
                "token",
                format!("ws://127.0.0.1:{ws_port}"),
            )
            .preferred_mode(VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize()),
        )
        .await
        .unwrap();

        let mut state_rx = handle.subscribe();
        handle
            .send(VoiceGatewayCommand::Heartbeat { nonce: 55 })
            .unwrap();
        heartbeat_rx.await.unwrap();
        handle.send_dave_transition_ready(99).unwrap();
        handle.send_dave_mls_key_package(vec![1, 2, 3]).unwrap();
        handle
            .send_dave_mls_commit_welcome(vec![4, 5], Some(vec![6, 7]))
            .unwrap();
        handle.send_dave_mls_invalid_commit_welcome(99).unwrap();
        dave_commands_rx.await.unwrap();

        timeout(Duration::from_secs(1), async {
            loop {
                let state = state_rx.borrow().clone();
                if state.resumed
                    && state.last_sequence == Some(25)
                    && state
                        .session_description
                        .as_ref()
                        .map(|description| description.secret_key.as_slice() == [9, 9])
                        == Some(true)
                    && state.dave.epoch == Some(2)
                    && state.dave.external_sender.as_deref() == Some([1, 2, 3].as_slice())
                {
                    break;
                }

                state_rx.changed().await.unwrap();
            }
        })
        .await
        .unwrap();

        let state = handle.state();
        assert_eq!(
            state.selected_mode,
            VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize()
        );
        assert_eq!(state.last_sequence, Some(25));
        assert!(state.resumed);
        assert_eq!(state.dave.protocol_version, Some(1));
        assert_eq!(state.dave.transition_id, Some(99));
        assert_eq!(state.dave.proposals, vec![vec![4, 5]]);
        assert_eq!(state.dave.pending_commit, Some(vec![6]));
        assert_eq!(state.dave.pending_welcome, Some(vec![7]));
        assert_eq!(
            state.session_description,
            Some(VoiceSessionDescription {
                mode: VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize(),
                secret_key: vec![9, 9],
                audio_codec: Some("opus".to_string()),
                dave_protocol_version: None,
            })
        );

        handle.close().await.unwrap();
        server.await.unwrap();
    }
}
