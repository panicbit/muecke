use anyhow::{Result, Error, ensure};
use tokio::io::{AsyncWrite, AsyncWriteExt};

use super::U16;

pub struct Str<'a>(&'a str);

impl Str<'static> {
    pub fn from_static(s: &'static str) -> Self {
        Self::try_from(s).unwrap()
    }
}

impl<'a> Str<'a> {
    pub const MAX_LEN: usize = 65_535;

    pub async fn write_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: AsyncWrite + Unpin + Send + Sync,
    {
        U16(self.len()).write_to(writer).await?;
        writer.write_all(self.0.as_bytes()).await?;

        Ok(())
    }

    pub fn len(&self) -> u16 {
        self.0.len() as u16
    }
}

impl<'a> TryFrom<&'a str> for Str<'a> {
    type Error = Error;

    fn try_from(s: &'a str) -> Result<Self> {
        ensure!(s.len() <= Self::MAX_LEN);

        Ok(Self(s))
    }
}
