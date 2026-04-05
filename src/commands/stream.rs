use std::net::{SocketAddr, UdpSocket};

use alsa::pcm::State;
use anyhow::{Context, Result};
use clap::Parser;
use socket2::{Domain, Protocol, Socket, Type};

use crate::alsa::open_capture;

const DEFAULT_PORT: u16 = 7373;
const DEFAULT_PERIOD: u64 = 1024;
const FRAME_SIZE: usize = size_of::<f32>() * 2;
const MAX_UDP_PAYLOAD: usize = 1472;

#[derive(Parser)]
#[command(about = "Capture IQ samples from ALSA and stream over UDP")]
pub struct Args {
    /// ALSA device name
    device: String,

    /// Destination IP address
    address: String,

    /// Destination UDP port
    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    port: u16,

    /// ALSA period size in frames
    #[arg(short = 'n', long, default_value_t = DEFAULT_PERIOD)]
    frames: u64,
}

pub fn run(args: Args) -> Result<()> {
    let pcm = open_capture(&args.device, args.frames)
        .with_context(|| format!("Failed to open ALSA device '{}'", args.device))?;

    let dest: SocketAddr = format!("{}:{}", args.address, args.port)
        .parse()
        .with_context(|| format!("Invalid address '{}'", args.address))?;

    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
        .context("Failed to create UDP socket")?;
    let send_buf_size = args.frames as usize * FRAME_SIZE * 4;
    socket
        .set_send_buffer_size(send_buf_size)
        .context("Failed to set SO_SNDBUF")?;
    socket.set_nonblocking(true)?;
    socket
        .connect(&dest.into())
        .with_context(|| format!("Failed to connect UDP socket to {dest}"))?;
    let socket: UdpSocket = socket.into();

    pcm.start().context("Failed to start PCM capture")?;

    let period_bytes = args.frames as usize * FRAME_SIZE;
    eprintln!(
        "Streaming IQ from '{}' to {} ({} frames/period, {} bytes/packet)",
        args.device, dest, args.frames, MAX_UDP_PAYLOAD
    );

    let io = pcm.io_f32().context("Failed to get PCM I/O handle")?;

    let mut buf = vec![0f32; args.frames as usize * 2];

    loop {
        if pcm.state() != State::Running {
            pcm.recover(32, true) // EPIPE
                .context("Failed to recover PCM")?;
            pcm.start().context("Failed to restart PCM")?;
        }

        pcm.wait(None).context("snd_pcm_wait failed")?;

        match io.readi(&mut buf) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("ALSA read error: {e}, recovering...");
                pcm.recover(e.errno() as i32, true)
                    .context("Failed to recover PCM after read error")?;
                continue;
            }
        }

        let data: &[u8] = bytemuck_cast_slice(&buf);
        for chunk in data[..period_bytes].chunks(MAX_UDP_PAYLOAD) {
            if let Err(e) = socket.send(chunk) {
                if e.kind() != std::io::ErrorKind::WouldBlock {
                    eprintln!("Send error: {e}");
                }
            }
        }
    }
}

fn bytemuck_cast_slice(floats: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(floats.as_ptr() as *const u8, floats.len() * 4) }
}

