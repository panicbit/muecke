use anyhow::{Error, Result, ensure};
use tokio::io::{AsyncWrite, AsyncWriteExt};


pub struct VarInt(u32);

impl VarInt {
    const MAX: Self = Self(268_435_455); // 2^28 - 1

    pub async fn write_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        let mut x = self.0;

        loop {
            let mut encoded_byte = (x % 128) as u8;

            x /= 128;

            if x > 0 {
                encoded_byte |= 128;
            }

            writer.write_u8(encoded_byte).await?;

            if x == 0 {
                break;
            }
        }

        Ok(())
    }
}

impl TryFrom<usize> for VarInt {
    type Error = Error;

    fn try_from(value: usize) -> Result<Self> {
        let value = u32::try_from(value)?;

        Self::try_from(value)
    }
}

impl TryFrom<u32> for VarInt {
    type Error = Error;

    fn try_from(value: u32) -> Result<Self> {
        ensure!(value <= Self::MAX.0);

        Ok(Self(value))
    }
}
