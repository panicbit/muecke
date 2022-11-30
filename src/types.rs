use anyhow::Result;
use async_trait::async_trait;
use tokio::io::{AsyncWrite, AsyncWriteExt};

mod var_int;
use self::var_int::VarInt;

mod str;
pub use self::str::Str;

#[repr(u8)]
pub enum ControlPacketKind {
    /// Reserved
    Reserved = 0,
    /// Connection request
    Connect = 1,
    /// Connect acknowledgment
    ConnAck = 2,
    /// Publish message
    Publish = 3,
    /// Publish acknowledgment (QoS 1)
    PubAck = 4,
    /// Publish received (QoS 2 delivery part 1)
    PubRec = 5,
    /// Publish release (QoS 2 delivery part 2)
    PubRel = 6,
    /// Publish complete (QoS 2 delivery part 3)
    PubComp = 7,
    /// Subscribe request
    Subscribe = 8,
    /// Subscribe acknowledgment
    SubAck = 9,
    /// Unsubscribe request
    Unsubscribe = 10,
    /// Unsubscribe acknowledgment
    UnsubAck = 11,
    /// PING request
    PingReq = 12,
    /// PING response
    PingResp = 13,
    /// Disconnect notification
    Disconnect = 14,
    /// Authentication exchange
    Auth = 15,
}

#[async_trait]
pub trait ControlPacket {
    const KIND: ControlPacketKind;

    fn flags(&self) -> u8 {
        0b0000
    }

    async fn write_variable_header_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: AsyncWrite + Unpin + Send + Sync;

    async fn write_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: AsyncWrite + Unpin + Send + Sync,
    {
        let kind = (Self::KIND as u8) << 4;
        let flags = self.flags() & 0b1111;
        let header_and_flags = kind | flags;
        let mut data = Vec::new();

        self.write_variable_header_to(&mut data).await?;
        self.write_payload_to(&mut data).await?;

        writer.write_u8(header_and_flags).await?;
        let data_len = VarInt::try_from(data.len())?;
        data_len.write_to(writer).await?;
        writer.write_all(&data).await?;

        writer.flush().await?;

        Ok(())
    }

    async fn write_payload_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: AsyncWrite + Unpin + Send + Sync;
}

pub struct Connect<'a> {
    pub clean_start: bool,
    pub will: Option<Will<'a>>,
    pub username: Option<Str<'a>>,
    pub password: Option<Str<'a>>,
    pub keep_alive: u16,
    pub client_id: Str<'a>,
}

#[async_trait]
impl<'a> ControlPacket for Connect<'a> {
    const KIND: ControlPacketKind = ControlPacketKind::Connect;

    async fn write_variable_header_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: AsyncWrite + Unpin + Send + Sync,
    {
        // Protocol Name
        Str::from_static("MQTT").write_to(writer).await?;

        // Protocol Version
        U8(5).write_to(writer).await?;

        // Connect Flags
        {
            let mut connect_flags = 0;

            connect_flags
                // Username
                .push(self.username.is_some())
                // Password
                .push(self.password.is_some())
                // Will Retain
                .push(self.will.as_ref().map(|will| will.retain).unwrap_or(false))
                // Will QoS
                .push2(self.will.as_ref().map(|will| will.qos as u8).unwrap_or(0))
                // Will Flag
                .push(self.will.is_some())
                // Clean Start
                .push(self.clean_start)
                // Reserved
                .push1(0);

            U8(connect_flags).write_to(writer).await?;
        }

        // Keep Alive
        U16(self.keep_alive).write_to(writer).await?;

        // Property Length (TODO)
        VarInt::try_from(0u32).unwrap().write_to(writer).await?;

        Ok(())
    }

    async fn write_payload_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: AsyncWrite + Unpin + Send + Sync,
    {
        self.client_id.write_to(writer).await?;

        // TODO

        Ok(())
    }
}

pub struct Will<'a> {
    message: Str<'a>,
    qos: QoS,
    retain: bool,
}

#[derive(Copy, Clone)]
pub enum QoS {
    QoS0 = 0,
    QoS1 = 1,
    QoS2 = 2,
}

#[derive(Copy, Clone)]
pub enum ConnectProperty {
    SessionExpiryInterval = 0x11,
    ReceiveMaximum = 0x21,
    MaximumPacketSize = 0x27,
    TopicAliasMaximum = 0x22,
    RequestResponseInformation = 0x19,
    RequestProblemInformation = 0x17,
    UserProperty = 0x26,
    AuthenticationMethod = 0x15,
    AuthenticationData = 0x16,
}

pub struct Publish<'a> {
    pub dup: bool,
    pub qos: QoS,
    pub identifier: u16,
    pub retain: bool,
    pub topic: Str<'a>,
    pub message: &'a [u8],
}

#[async_trait]
impl<'a> ControlPacket for Publish<'a> {
    const KIND: ControlPacketKind = ControlPacketKind::Publish;

    fn flags(&self) -> u8 {
        let mut flags = 0;

        flags.push(self.dup).push2(self.qos as u8).push(self.retain);

        flags
    }

    async fn write_variable_header_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: AsyncWrite + Unpin + Send + Sync,
    {
        // Topic Name
        self.topic.write_to(writer).await?;

        // Packet Identifier
        if let QoS::QoS1 | QoS::QoS2 = self.qos {
            U16(self.identifier).write_to(writer).await?;
        }

        // Property Length (TODO)
        VarInt::try_from(0u32).unwrap().write_to(writer).await?;

        Ok(())
    }

    async fn write_payload_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: AsyncWrite + Unpin + Send + Sync,
    {
        writer.write_all(self.message).await?;

        Ok(())
    }
}

struct U8(u8);

impl U8 {
    async fn write_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: AsyncWrite + Unpin + Send + Sync,
    {
        writer.write_u8(self.0).await?;
        Ok(())
    }
}

struct U16(u16);

impl U16 {
    async fn write_to<W>(&self, writer: &mut W) -> Result<()>
    where
        W: AsyncWrite + Unpin + Send + Sync,
    {
        writer.write_u16(self.0).await?;
        Ok(())
    }
}

trait BitVec {
    fn push(&mut self, value: bool) -> &mut Self;
    fn push1(&mut self, value: u8) -> &mut Self;
    fn push2(&mut self, value: u8) -> &mut Self;
}

impl BitVec for u8 {
    fn push(&mut self, value: bool) -> &mut Self {
        self.push1(value as u8)
    }

    fn push1(&mut self, value: u8) -> &mut Self {
        *self = (*self << 1) | (value & 0b1);
        self
    }

    fn push2(&mut self, value: u8) -> &mut Self {
        *self = (*self << 2) | (value & 0b11);
        self
    }
}
