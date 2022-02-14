// Copyright 2022 The Engula Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use async_trait::async_trait;
use futures::Stream;
use tonic::{transport::Channel, Streaming};

use super::StreamMeta;
use crate::{masterpb, Error, Result, SegmentMeta};

impl From<masterpb::StreamMeta> for super::StreamMeta {
    fn from(meta: masterpb::StreamMeta) -> Self {
        super::StreamMeta {
            stream_id: meta.stream_id,
            stream_name: meta.stream_name,
        }
    }
}

impl From<masterpb::Command> for super::Command {
    fn from(cmd: masterpb::Command) -> Self {
        match masterpb::CommandType::from_i32(cmd.command_type) {
            None | Some(masterpb::CommandType::Nop) => super::Command::Nop,
            Some(masterpb::CommandType::Promote) => super::Command::Promote {
                role: cmd.role.into(),
                epoch: cmd.epoch,
                leader: cmd.leader,
                pending_epochs: cmd.pending_epochs,
            },
        }
    }
}

impl From<super::Command> for masterpb::Command {
    fn from(cmd: super::Command) -> Self {
        match cmd {
            super::Command::Nop => masterpb::Command {
                command_type: masterpb::CommandType::Nop as i32,
                epoch: 0,
                role: crate::Role::Follower.into(),
                leader: "".into(),
                pending_epochs: Vec::default(),
            },
            super::Command::Promote {
                role,
                epoch,
                leader,
                pending_epochs,
            } => masterpb::Command {
                command_type: masterpb::CommandType::Promote as i32,
                epoch,
                role: role.into(),
                leader,
                pending_epochs,
            },
        }
    }
}

type MasterClient = masterpb::master_client::MasterClient<Channel>;

#[derive(Clone)]
#[allow(unused)]
pub(super) struct Client {
    client: MasterClient,
}

#[allow(dead_code)]
impl Client {
    pub async fn connect(addr: &str) -> crate::Result<Self> {
        let addr = format!("http://{}", addr);
        let client = MasterClient::connect(addr).await?;
        Ok(Client { client })
    }

    pub async fn create_stream(
        &self,
        input: masterpb::CreateStreamRequest,
    ) -> crate::Result<masterpb::CreateStreamResponse> {
        let mut client = self.client.clone();
        let resp = client.create_stream(input).await?;
        Ok(resp.into_inner())
    }

    pub async fn list_stream(
        &self,
        input: masterpb::ListStreamRequest,
    ) -> crate::Result<Streaming<masterpb::StreamMeta>> {
        let mut client = self.client.clone();
        Ok(client.list_stream(input).await?.into_inner())
    }

    pub async fn get_stream(
        &self,
        input: masterpb::GetStreamRequest,
    ) -> crate::Result<masterpb::GetStreamResponse> {
        let mut client = self.client.clone();
        let resp = client.get_stream(input).await?;
        Ok(resp.into_inner())
    }

    pub async fn get_segment(
        &self,
        input: masterpb::GetSegmentRequest,
    ) -> crate::Result<masterpb::GetSegmentResponse> {
        let mut client = self.client.clone();
        let resp = client.get_segment(input).await?;
        Ok(resp.into_inner())
    }

    pub async fn heartbeat(
        &self,
        input: masterpb::HeartbeatRequest,
    ) -> crate::Result<masterpb::HeartbeatResponse> {
        let mut client = self.client.clone();
        let resp = client.heartbeat(input).await?;
        Ok(resp.into_inner())
    }

    pub async fn seal_segment(
        &self,
        input: masterpb::SealSegmentRequest,
    ) -> crate::Result<masterpb::SealSegmentResponse> {
        let mut client = self.client.clone();
        let resp = client.seal_segment(input).await?;
        Ok(resp.into_inner())
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct RemoteMaster {
    master_client: Client,
}

#[allow(dead_code)]
impl RemoteMaster {
    pub async fn new(addr: &str) -> Result<Self> {
        Ok(RemoteMaster {
            master_client: Client::connect(addr).await?,
        })
    }
}

pub struct MetaStream {}

impl Stream for MetaStream {
    type Item = Result<SegmentMeta>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        todo!()
    }
}

pub struct ListStream {
    streaming: Streaming<masterpb::StreamMeta>,
}

impl Stream for ListStream {
    type Item = Result<StreamMeta>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.get_mut().streaming).poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(item)) => match item {
                Ok(meta) => Poll::Ready(Some(Ok(meta.into()))),
                Err(err) => Poll::Ready(Some(Err(err.into()))),
            },
        }
    }
}

#[async_trait]
#[allow(dead_code, unused)]
impl super::Master for RemoteMaster {
    type ListStream = ListStream;
    type MetaStream = MetaStream;

    async fn create_stream(&self, stream_name: &str) -> Result<()> {
        let req = masterpb::CreateStreamRequest {
            stream_name: stream_name.to_string(),
        };

        self.master_client.create_stream(req).await?;
        Ok(())
    }

    async fn list_stream(&self) -> Result<Self::ListStream> {
        Ok(ListStream {
            streaming: self
                .master_client
                .list_stream(masterpb::ListStreamRequest {})
                .await?,
        })
    }

    async fn get_stream(&self, stream_name: &str) -> Result<Option<StreamMeta>> {
        let req = masterpb::GetStreamRequest {
            stream_name: stream_name.to_string(),
        };

        match self.master_client.get_stream(req).await {
            Ok(resp) => Ok(Some(StreamMeta {
                stream_id: resp.stream_id,
                stream_name: stream_name.to_owned(),
            })),
            Err(Error::NotFound(_)) => Ok(None),
            Err(err) => Err(err),
        }
    }

    async fn heartbeat(&self, observer_meta: super::ObserverMeta) -> Result<Vec<super::Command>> {
        let req = masterpb::HeartbeatRequest {
            epoch: observer_meta.epoch,
            observer_id: observer_meta.observer_id,
            stream_name: observer_meta.stream_name,
            role: observer_meta.state.role().into(),
            observer_state: observer_meta.state.into(),
            acked_seq: observer_meta.acked_seq.into(),
        };

        let resp = self.master_client.heartbeat(req).await?;
        Ok(resp.commands.into_iter().map(Into::into).collect())
    }

    async fn query_segments(
        &self,
        stream_name: &str,
        range: std::ops::Range<u64>,
    ) -> Result<Self::MetaStream> {
        todo!()
    }

    async fn get_segment(&self, stream_name: &str, epoch: u32) -> Result<Option<SegmentMeta>> {
        let req = masterpb::GetSegmentRequest {
            stream_name: stream_name.to_owned(),
            seg_epoch: epoch,
        };

        let resp = match self.master_client.get_segment(req).await {
            Err(crate::Error::NotFound(_)) => return Ok(None),
            Err(e) => return Err(e),
            Ok(resp) => resp,
        };

        let seg_meta = SegmentMeta {
            stream_id: resp.stream_id,
            stream_name: stream_name.to_owned(),
            epoch,
            copy_set: resp.copy_set,
            state: resp.state.into(),
        };

        Ok(Some(seg_meta))
    }

    async fn seal_segment(&self, stream_id: u64) -> Result<()> {
        let req = masterpb::SealSegmentRequest { stream_id };
        self.master_client.seal_segment(req).await?;
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        journal::master::{Command, Master, ObserverMeta, ObserverState},
        servers::master::build_master,
        Role, SegmentState, Sequence,
    };

    #[tokio::test(flavor = "multi_thread")]
    async fn create_stream() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let master_addr = build_master(&[]).await?;

        let master = RemoteMaster::new(&master_addr).await?;
        match master.get_segment("test", 1).await? {
            None => {}
            Some(_) => panic!("no segment named `test` exists"),
        }

        master.create_stream("test").await?;
        match master.get_segment("test", 1).await? {
            Some(_) => {}
            None => panic!("segment named `test` must exists"),
        }

        match master.create_stream("test").await {
            Err(crate::Error::AlreadyExists(_)) => {}
            _ => panic!("create same stream must fail"),
        }

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_segment() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let replicas = vec!["a", "b", "c"];
        let local_addr = build_master(&replicas).await?;
        let master = RemoteMaster::new(&local_addr.to_string()).await?;
        let resp = master.get_segment("default", 1).await?;
        assert!(
            matches!(resp, Some(segment_meta) if segment_meta == SegmentMeta {
                stream_id: 1,
                stream_name: "default".to_owned(),
                epoch: 1,
                copy_set: replicas.iter().map(ToString::to_string).collect(),
                state: SegmentState::Appending,
            })
        );

        let resp = master.get_segment("not-exists", 1).await?;
        assert!(matches!(resp, None));

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn heartbeat() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let replicas = vec!["a", "b", "c"];
        let local_addr = build_master(&replicas).await?;
        let master = RemoteMaster::new(&local_addr.to_string()).await?;
        let observer_meta = ObserverMeta {
            observer_id: "1".to_owned(),
            stream_name: "default".to_owned(),
            epoch: 1,
            state: ObserverState::Leading,
            acked_seq: Sequence::new(1, 0),
        };
        master.heartbeat(observer_meta).await?;
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn heartbeat_with_threshold_switching(
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        use crate::servers::master::mem::DEFAULT_NUM_THRESHOLD;

        let replicas = vec!["a", "b", "c"];
        let local_addr = build_master(&replicas).await?;
        let master = RemoteMaster::new(&local_addr.to_string()).await?;
        let observer_meta = ObserverMeta {
            observer_id: "1".to_owned(),
            stream_name: "default".to_owned(),
            epoch: 0,
            state: ObserverState::Leading,
            acked_seq: ((DEFAULT_NUM_THRESHOLD + 1) as u64).into(),
        };
        let commands = master.heartbeat(observer_meta).await?;
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            &commands[0],
            Command::Promote {
                role,
                epoch,
                leader: _,
                pending_epochs: _,
            }
            if *epoch == 1 && *role == Role::Leader
        ));
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn seal_segment() -> Result<()> {
        let master_addr = build_master(&[]).await?;
        let master = RemoteMaster::new(&master_addr).await?;
        let meta = master.get_segment("default", 1).await?.unwrap();
        assert_eq!(meta.state, SegmentState::Appending);

        master.seal_segment(meta.stream_id).await?;
        let meta = master.get_segment("default", 1).await?.unwrap();
        assert_eq!(meta.state, SegmentState::Sealed);

        Ok(())
    }
}