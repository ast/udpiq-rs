use alsa::pcm::{Access, Format, HwParams, PCM};
use alsa::ValueOr;
use thiserror::Error;

const SAMPLE_RATE: u32 = 78125;
const CHANNELS: u32 = 2;
const PERIODS: u32 = 4;

#[derive(Debug, Error)]
pub enum AlsaError {
    #[error("ALSA error: {0}")]
    Alsa(#[from] alsa::Error),
}

pub fn open_capture(device: &str, period_size: u64) -> Result<PCM, AlsaError> {
    let pcm = PCM::new(device, alsa::Direction::Capture, false)?;

    {
        let hwp = HwParams::any(&pcm)?;
        hwp.set_access(Access::MMapInterleaved)?;
        hwp.set_format(Format::float())?;
        hwp.set_channels(CHANNELS)?;
        hwp.set_rate(SAMPLE_RATE, ValueOr::Nearest)?;
        hwp.set_period_size(period_size as i64, ValueOr::Nearest)?;
        hwp.set_periods(PERIODS, ValueOr::Nearest)?;
        pcm.hw_params(&hwp)?;
    }

    Ok(pcm)
}
