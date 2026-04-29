# Voice

Voice is an optional runtime layer. It stays feature-gated so core Gateway, REST, and interaction code do not pay for voice dependencies.

## Enable

```toml
[dependencies]
discordrs = { version = "1.2.0", features = ["voice"] }

# PCM -> Opus encode/playback helpers
discordrs = { version = "1.2.0", features = ["voice", "voice-encode"] }

# Experimental DAVE/MLS hook
discordrs = { version = "1.2.0", features = ["voice", "dave"] }
```

## Surfaces

- `VoiceManager`: tracks gateway voice state/server updates and local queue state.
- `connect_voice_runtime(...)`: connects the voice websocket, performs UDP discovery, selects protocol, and waits for session description.
- `recv_raw_udp_packet(...)`: receives raw UDP packets with parsed RTP metadata.
- `recv_voice_packet(...)`: returns transport-decrypted Opus frames for non-DAVE sessions.
- `send_opus_frame(...)`: sends an already-encoded Opus frame as an encrypted RTP-size packet with managed sequence, timestamp, and nonce state.
- `play_opus_frames(...)`: sends a small iterator of encoded Opus frames with simple duration pacing and speaking on/off commands.
- `PcmFrame`, `AudioSource`, `AudioMixer`, and `VoiceOpusEncoder`: behind `voice-encode`, encode 48 kHz stereo 20 ms PCM frames with `opus-rs` Audio mode before calling the existing Opus playback path.
- `VoiceOpusDecoder`: decodes Opus frames to interleaved `i16` PCM, using 48 kHz stereo by default for Discord voice.
- `VoiceDaveFrameDecryptor`: trait for DAVE frame decryptors.
- `VoiceDaveFrameEncryptor` and `send_opus_frame_with_dave(...)`: behind `dave`, encrypt Opus with DAVE before RTP packetization.
- `VoiceDaveySession`: experimental `dave` feature wrapper over `davey` / OpenMLS. `VoiceDaveyDecryptor` remains as a compatibility alias.

## Example

```rust
use std::time::Duration;

use discordrs::{connect_voice_runtime, VoiceOpusDecoder, VoiceRuntimeConfig};

async fn receive_voice() -> Result<(), discordrs::DiscordError> {
    let handle = connect_voice_runtime(VoiceRuntimeConfig::new(
        "guild_id",
        "bot_user_id",
        "voice_session_id",
        "voice_token",
        "wss://voice.discord.media/?v=8",
    ))
    .await?;

    handle
        .send_opus_frame(&[0xf8, 0xff, 0xfe], Duration::from_millis(20))
        .await?;

    let mut decoder = VoiceOpusDecoder::discord_default()?;
    let decoded = handle.recv_decoded_voice_packet(&mut decoder, 2048).await?;
    println!(
        "SSRC {} produced {} samples/channel",
        decoded.packet.rtp.ssrc,
        decoded.samples_per_channel
    );

    handle.close().await
}
```

## DAVE Boundary

Default `voice` can send already-encoded Opus frames, decrypt Discord voice transport encryption, and decode received Opus to PCM. It does not claim full DAVE/MLS interoperability by itself.

For active DAVE sessions, use `recv_voice_packet_with_dave(...)` or `recv_decoded_voice_packet_with_dave(...)` with a `VoiceDaveFrameDecryptor`. The `dave` feature provides `VoiceDaveySession` for MLS lifecycle entry points and `send_opus_frame_with_dave(...)` for the outbound media insertion point. Plain `send_opus_frame(...)` and `play_opus_frames(...)` intentionally reject active DAVE sessions so callers do not accidentally bypass end-to-end media encryption.

The live transition harness is `tests/live_voice_dave.rs` and is ignored by default. Run it only against a prepared Discord voice session:

```powershell
$env:DISCORDRS_LIVE_VOICE_SERVER_ID="..."
$env:DISCORDRS_LIVE_VOICE_USER_ID="..."
$env:DISCORDRS_LIVE_VOICE_SESSION_ID="..."
$env:DISCORDRS_LIVE_VOICE_TOKEN="..."
$env:DISCORDRS_LIVE_VOICE_ENDPOINT="..."
$env:DISCORDRS_LIVE_VOICE_CHANNEL_ID="..."
cargo test --all-features --test live_voice_dave -- --ignored
```
