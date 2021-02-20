use super::errors::ClientError;

use crate::chunk::define::{chunk_type, csid_type};
use crate::chunk::unpacketizer::ChunkUnpacketizer;
use crate::chunk::unpacketizer::UnpackResult;
use crate::chunk::{packetizer::ChunkPacketizer, ChunkInfo};

use crate::handshake::handshake::SimpleHandshakeClient;

use crate::messages::define::msg_type;
use crate::messages::processor::MessageProcessor;

use liverust_lib::netio::bytes_writer::AsyncBytesWriter;
use liverust_lib::netio::bytes_writer::BytesWriter;
use liverust_lib::netio::netio::NetworkIO;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::netconnection::commands::ConnectProperties;
use crate::netconnection::commands::NetConnection;
use crate::netstream::commands::NetStream;
use crate::protocol_control_messages::control_messages::ControlMessages;
use crate::user_control_messages::errors::EventMessagesError;
use crate::user_control_messages::event_messages::EventMessages;

// use std::collections::HashMap;

use tokio::{prelude::*, stream::StreamExt, time::timeout};
use tokio_util::codec::{BytesCodec, Framed};
pub struct ClientSession<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    packetizer: ChunkPacketizer<S>,
    unpacketizer: ChunkUnpacketizer,
    handshaker: SimpleHandshakeClient<S>,
    io: Rc<RefCell<NetworkIO<S>>>,

    state: ClientSessionState,
}

enum ClientSessionState {
    Handshake,
    ReadChunk,
}

impl<S> ClientSession<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn new(stream: S, timeout: Duration) -> Self {
        let net_io = Rc::new(RefCell::new(NetworkIO::new(stream, timeout)));
        let bytes_writer = AsyncBytesWriter::new(net_io.clone());

        Self {
            io: net_io.clone(),

            packetizer: ChunkPacketizer::new(bytes_writer),
            unpacketizer: ChunkUnpacketizer::new(),
            handshaker: SimpleHandshakeClient::new(net_io.clone()),

            state: ClientSessionState::Handshake,
        }
    }

    pub async fn run(&mut self) -> Result<(), ClientError> {
        loop {
            //let data = self.io.borrow_mut().read().await?;

            let data = self.io.borrow_mut().read().await?;
            match self.state {
                ClientSessionState::Handshake => {
                    self.handshaker.extend_data(&data[..]);
                    let result = self.handshaker.handshake().await;

                    match result {
                        Ok(_) => {
                            self.state = ClientSessionState::ReadChunk;
                        }
                        Err(_) => {}
                    }
                }
                ClientSessionState::ReadChunk => {
                    self.unpacketizer.extend_data(&data[..]);
                    let result = self.unpacketizer.read_chunk()?;

                    match result {
                        UnpackResult::ChunkInfo(chunk_info) => {
                            let mut message_parser = MessageProcessor::new(chunk_info);
                            let mut rtmp_msg = message_parser.execute()?;

                            // self.process_rtmp_message(&mut rtmp_msg)?;
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    pub fn send_connect(&mut self, transaction_id: &f64) -> Result<(), ClientError> {
        let app_name = String::from("app");
        let properties = ConnectProperties::new(app_name);

        let mut netconnection = NetConnection::new(BytesWriter::new());
        let r = netconnection.connect(transaction_id, &properties)?;
        //self.packetizer.
        Ok(())
    }

    pub fn send_create_stream(&mut self, transaction_id: &f64) -> Result<(), ClientError> {
        let mut netconnection = NetConnection::new(BytesWriter::new());
        let data = netconnection.create_stream(transaction_id)?;

        let mut chunk_info = ChunkInfo::new(
            csid_type::COMMAND_AMF0_AMF3,
            chunk_type::TYPE_0,
            0,
            data.len() as u32,
            msg_type::COMMAND_AMF0,
            0,
            data,
        );

        self.packetizer.write_chunk(&mut chunk_info)?;

        Ok(())
    }

    pub fn send_delete_stream(
        &mut self,
        transaction_id: &f64,
        stream_id: &f64,
    ) -> Result<(), ClientError> {
        let mut netstream = NetStream::new(BytesWriter::new());
        netstream.delete_stream(transaction_id, stream_id)?;
        Ok(())
    }

    pub fn send_publish(
        &mut self,
        transaction_id: &f64,
        stream_name: &String,
        stream_type: &String,
    ) -> Result<(), ClientError> {
        let mut netstream = NetStream::new(BytesWriter::new());
        netstream.publish(transaction_id, stream_name, stream_type)?;
        Ok(())
    }

    //     pub fn send_play(&mut self)-> Result<(), ClientError> {
    //     }
}