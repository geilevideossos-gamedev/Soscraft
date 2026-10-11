use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bevy::prelude::*;
use master_protocol::{
    MAX_BOOTSTRAP_STREAM_BYTES, MemberId, RelayDatagram, decode_relay_stream, encode_relay_stream,
};
use sim::ClientId;
use tokio_util::sync::CancellationToken;

use crate::authority::runtime::AuthorityWorld;
use crate::transport::master::{MasterBridge, MasterBridgeState, TransportFault};
use crate::transport::protocol::MAX_PACKET_BYTES;
use crate::transport::udp_session::{UdpAuthorityHub, UdpClientLink};

pub const HOST: u32 = 0;
pub const MC_MAGIC: [u8; 2] = *b"MC";
pub const FRAME_BYTES: usize = 15 * 1024;
pub const MESSAGE_LIMIT: usize = 4 * 1024 * 1024;
pub const STATE_LIMIT: usize = 256 * 1024;
pub const MAILBOX_LIMIT: usize = 48;
pub const HOST_WINDOW: u32 = 8;
pub const WINDOW_TOTAL: u32 = 24;
pub const FRIEND_WINDOW: u32 = 4;
pub const RELIABLE_BUDGET: usize = 64 * 1024;
pub const BACKLOG_LIMIT: usize = 4 * 1024 * 1024;
pub const ACK_TIMEOUT: Duration = Duration::from_secs(20);
pub const BULK_STREAMS: usize = 2;
pub const BATCH_RESEND: Duration = Duration::from_secs(3);
pub const BATCH_GIVE_UP: Duration = Duration::from_secs(30);
pub const BAD_LIMIT: usize = 20;
pub const BAD_WINDOW: Duration = Duration::from_secs(60);
pub const TICK: Duration = Duration::from_millis(50);
pub const TOO_SLOW: &str = "connection too slow";
pub const LOST_STREAM: &str = "Lost the world stream";
pub const BROKEN_MESSAGES: &str = "Too many broken messages";

const FRAME_HEADER: usize = 8;
const PACK_HEADER: usize = 5;
const RAW_CHUNK_MAX: usize = 4 * FRAME_BYTES;
const STATE_PACKET_BYTES: usize = MAX_PACKET_BYTES as usize - 64;
const STATE_INBOX: usize = 64;
const BULK_FAILURES: usize = 64;
const BULK_OUTBOUND_CAP: usize = 16;
const BULK_INBOUND_CAP: usize = 16;
const STREAM_DEADLINE: Duration = Duration::from_secs(8);
const SEND_DEADLINE: Duration = Duration::from_secs(10);
const BULK_PRIORITY: i32 = -48;
const BUDGET_PERIOD: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinecraftLane {
    Reliable,
    State,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WireRole {
    Host,
    Friend,
}

pub trait MinecraftLink {
    fn control_queued(&self) -> usize;
    fn send_minecraft_to(&mut self, peer: u32, lane: MinecraftLane, payload: Vec<u8>) -> bool;
    fn send_bulk_to(&mut self, peer: u32, seq: u32, blob: Arc<Vec<u8>>) -> bool;
}

impl MinecraftLink for UdpAuthorityHub {
    fn control_queued(&self) -> usize {
        self.mailbox().control_outbound_len()
    }

    fn send_minecraft_to(&mut self, peer: u32, lane: MinecraftLane, payload: Vec<u8>) -> bool {
        match self.send_minecraft(ClientId(peer), lane, payload) {
            Ok(()) => true,
            Err(error) => {
                diag::warn!(Net, "minecraft packet to client {peer} not queued: {error}");
                false
            }
        }
    }

    fn send_bulk_to(&mut self, peer: u32, seq: u32, blob: Arc<Vec<u8>>) -> bool {
        let Some(member) = self.member_of_client(ClientId(peer)) else {
            return false;
        };
        match self
            .mailbox()
            .minecraft_bulk()
            .push_outbound(member, seq, blob)
        {
            Ok(()) => true,
            Err(error) => {
                diag::warn!(
                    Net,
                    "minecraft batch {seq} to client {peer} not queued: {error}"
                );
                false
            }
        }
    }
}

impl MinecraftLink for UdpClientLink {
    fn control_queued(&self) -> usize {
        self.mailbox().control_outbound_len()
    }

    fn send_minecraft_to(&mut self, _peer: u32, lane: MinecraftLane, payload: Vec<u8>) -> bool {
        match self.send_minecraft(lane, payload) {
            Ok(()) => true,
            Err(error) => {
                diag::warn!(Net, "minecraft packet to the host not queued: {error}");
                false
            }
        }
    }

    fn send_bulk_to(&mut self, _peer: u32, _seq: u32, _blob: Arc<Vec<u8>>) -> bool {
        false
    }
}

#[derive(Clone, Debug)]
pub struct BulkJob {
    pub member: MemberId,
    pub seq: u32,
    pub blob: Arc<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BulkReport {
    pub member: MemberId,
    pub seq: u32,
    pub result: Result<(), String>,
}

#[derive(Debug, Default)]
pub struct MinecraftBulk {
    outbound: Mutex<VecDeque<BulkJob>>,
    outbound_ready: tokio::sync::Notify,
    reports: Mutex<Vec<BulkReport>>,
    inbound: Mutex<VecDeque<Vec<u8>>>,
}

impl MinecraftBulk {
    pub fn push_outbound(
        &self,
        member: MemberId,
        seq: u32,
        blob: Arc<Vec<u8>>,
    ) -> Result<(), &'static str> {
        let mut queue = self.outbound.lock().expect("minecraft bulk poisoned");
        if queue.len() >= BULK_OUTBOUND_CAP {
            return Err("minecraft bulk queue is full");
        }
        queue.push_back(BulkJob { member, seq, blob });
        drop(queue);
        self.outbound_ready.notify_one();
        Ok(())
    }

    pub fn take_outbound(&self) -> Vec<BulkJob> {
        self.outbound
            .lock()
            .expect("minecraft bulk poisoned")
            .drain(..)
            .collect()
    }

    pub fn report(&self, member: MemberId, seq: u32, result: Result<(), String>) {
        self.reports
            .lock()
            .expect("minecraft bulk poisoned")
            .push(BulkReport {
                member,
                seq,
                result,
            });
    }

    pub fn take_reports(&self) -> Vec<BulkReport> {
        std::mem::take(&mut *self.reports.lock().expect("minecraft bulk poisoned"))
    }

    pub fn push_inbound(&self, blob: Vec<u8>) -> Result<(), &'static str> {
        let mut queue = self.inbound.lock().expect("minecraft bulk poisoned");
        if queue.len() >= BULK_INBOUND_CAP {
            return Err("minecraft stream queue is full");
        }
        queue.push_back(blob);
        Ok(())
    }

    pub fn take_inbound(&self) -> Vec<Vec<u8>> {
        self.inbound
            .lock()
            .expect("minecraft bulk poisoned")
            .drain(..)
            .collect()
    }
}

pub trait UniStream: Send + 'static {
    fn read_exact(&mut self, buf: &mut [u8]) -> impl Future<Output = Result<(), String>> + Send;
    fn read_to_end(&mut self, limit: usize)
    -> impl Future<Output = Result<Vec<u8>, String>> + Send;
}

impl UniStream for quinn::RecvStream {
    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), String> {
        quinn::RecvStream::read_exact(self, buf)
            .await
            .map_err(|error| error.to_string())
    }

    async fn read_to_end(&mut self, limit: usize) -> Result<Vec<u8>, String> {
        quinn::RecvStream::read_to_end(self, limit)
            .await
            .map_err(|error| error.to_string())
    }
}

pub async fn read_uni_prefix<R: UniStream>(recv: &mut R) -> Result<Vec<u8>, String> {
    let mut prefix = vec![0; 2];
    recv.read_exact(&mut prefix).await?;
    let header = if matches!(prefix[1], 2 | 3) { 18 } else { 2 };
    prefix.resize(header + MC_MAGIC.len(), 0);
    recv.read_exact(&mut prefix[2..]).await?;
    Ok(prefix)
}

pub fn is_minecraft_stream(prefix: &[u8]) -> bool {
    prefix.ends_with(&MC_MAGIC)
}

pub async fn read_minecraft_stream<R: UniStream>(
    mut recv: R,
    prefix: Vec<u8>,
    bulk: Arc<MinecraftBulk>,
    role: &'static str,
) {
    let limit = MAX_BOOTSTRAP_STREAM_BYTES.saturating_sub(prefix.len());
    let rest = match tokio::time::timeout(STREAM_DEADLINE, recv.read_to_end(limit)).await {
        Ok(Ok(rest)) => rest,
        Ok(Err(error)) => {
            diag::warn!(Net, "master {role} dropped a minecraft stream: {error}");
            return;
        }
        Err(_) => {
            diag::warn!(Net, "master {role} dropped a minecraft stream: deadline");
            return;
        }
    };
    let mut bytes = prefix;
    bytes.extend_from_slice(&rest);
    match decode_relay_stream(&bytes) {
        Ok(
            RelayDatagram::ServiceToMember(payload) | RelayDatagram::HostToMember { payload, .. },
        ) => {
            if let Err(error) = bulk.push_inbound(payload.to_vec()) {
                diag::warn!(Net, "master {role} dropped a minecraft stream: {error}");
            }
        }
        Ok(other) => {
            diag::warn!(
                Net,
                "master {role} ignored unexpected minecraft uni {other:?}"
            );
        }
        Err(error) => {
            diag::warn!(Net, "master {role} dropped a minecraft stream: {error}");
        }
    }
}

pub trait BulkSender: Clone + Send + Sync + 'static {
    fn send_bulk(
        &self,
        member: MemberId,
        blob: Arc<Vec<u8>>,
    ) -> impl Future<Output = Result<(), String>> + Send;
}

impl BulkSender for quinn::Connection {
    async fn send_bulk(&self, member: MemberId, blob: Arc<Vec<u8>>) -> Result<(), String> {
        let bytes = encode_relay_stream(RelayDatagram::HostToMember {
            member_id: member,
            payload: &blob[..],
        })
        .map_err(|error| error.to_string())?;
        let send = async {
            let mut stream = self.open_uni().await.map_err(|error| error.to_string())?;
            stream
                .set_priority(BULK_PRIORITY)
                .map_err(|error| error.to_string())?;
            stream
                .write_all(&bytes)
                .await
                .map_err(|error| error.to_string())?;
            stream.finish().map_err(|error| error.to_string())?;
            match stream.stopped().await {
                Ok(None) => Ok(()),
                Ok(Some(code)) => Err(format!("the relay refused the stream (code {code})")),
                Err(error) => Err(error.to_string()),
            }
        };
        tokio::time::timeout(SEND_DEADLINE, send)
            .await
            .map_err(|_| "deadline".to_owned())?
    }
}

pub async fn minecraft_egress<S: BulkSender>(
    sender: S,
    bulk: Arc<MinecraftBulk>,
    cancel: CancellationToken,
) -> Result<(), TransportFault> {
    let mut sends = tokio::task::JoinSet::new();
    loop {
        for job in bulk.take_outbound() {
            let sender = sender.clone();
            let bulk = Arc::clone(&bulk);
            sends.spawn(async move {
                let result = sender.send_bulk(job.member, job.blob).await;
                if let Err(error) = &result {
                    diag::warn!(Net, "minecraft batch {} not sent: {error}", job.seq);
                }
                bulk.report(job.member, job.seq, result);
            });
        }
        tokio::select! {
            _ = cancel.cancelled() => return Ok(()),
            _ = bulk.outbound_ready.notified() => {}
            Some(_) = sends.join_next(), if !sends.is_empty() => {}
        }
    }
}

fn compress(raw: &[u8]) -> Option<Vec<u8>> {
    zstd::bulk::compress(raw, 1).ok()
}

fn pack_compressed(raw_len: usize, compressed: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(PACK_HEADER + compressed.len());
    out.push(1);
    out.extend_from_slice(&(raw_len as u32).to_be_bytes());
    out.extend_from_slice(&compressed);
    out
}

fn pack(raw: &[u8]) -> Vec<u8> {
    match compress(raw) {
        Some(compressed) if compressed.len() + 4 < raw.len() => {
            pack_compressed(raw.len(), compressed)
        }
        _ => {
            let mut out = Vec::with_capacity(1 + raw.len());
            out.push(0);
            out.extend_from_slice(raw);
            out
        }
    }
}

fn unpack(bytes: &[u8], limit: usize) -> Option<Vec<u8>> {
    let (&flag, rest) = bytes.split_first()?;
    match flag {
        0 => (rest.len() <= limit).then(|| rest.to_vec()),
        1 => {
            let len = u32::from_be_bytes(rest.get(..4)?.try_into().ok()?) as usize;
            if len > limit {
                return None;
            }
            let raw = zstd::bulk::decompress(&rest[4..], len).ok()?;
            (raw.len() == len).then_some(raw)
        }
        _ => None,
    }
}

fn read_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(bytes[at..at + 4].try_into().expect("length checked"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrameFault {
    Broken,
    Lost,
}

#[derive(Debug)]
struct Batch {
    blob: Arc<Vec<u8>>,
    first: Option<Instant>,
    last: Option<Instant>,
    sending: bool,
}

#[derive(Debug, Default)]
struct Peer {
    out: VecDeque<u8>,
    next_frame: u32,
    acked: u32,
    received: u32,
    ack_sent: u32,
    ack_wait: Option<Instant>,
    budget_from: Option<Instant>,
    budget_used: usize,
    inbound: Vec<u8>,
    bad: VecDeque<Instant>,
    state: Option<Vec<u8>>,
    batches: BTreeMap<u32, Batch>,
}

impl Peer {
    fn new() -> Self {
        Self {
            next_frame: 1,
            ..Self::default()
        }
    }

    fn unacked(&self) -> u32 {
        self.next_frame - 1 - self.acked
    }

    fn queue(&mut self, message: &[u8]) {
        self.out
            .extend((message.len() as u32).to_be_bytes().iter().copied());
        self.out.extend(message.iter().copied());
    }

    fn budget_left(&mut self, now: Instant, budget: usize) -> usize {
        if self
            .budget_from
            .is_none_or(|from| now.duration_since(from) >= BUDGET_PERIOD)
        {
            self.budget_from = Some(now);
            self.budget_used = 0;
        }
        budget.saturating_sub(self.budget_used)
    }

    fn data_frame(&mut self) -> (Vec<u8>, usize) {
        let pending = self.out.make_contiguous();
        let raw_cap = FRAME_BYTES - FRAME_HEADER - PACK_HEADER;
        let mut chosen = None;
        for take in [RAW_CHUNK_MAX, RAW_CHUNK_MAX / 2] {
            let take = take.min(pending.len());
            if take <= raw_cap {
                break;
            }
            if let Some(compressed) = compress(&pending[..take])
                && compressed.len() <= raw_cap
            {
                chosen = Some((pack_compressed(take, compressed), take));
                break;
            }
        }
        let (body, used) = chosen.unwrap_or_else(|| {
            let take = raw_cap.min(pending.len());
            (pack(&pending[..take]), take)
        });
        let mut frame = Vec::with_capacity(FRAME_HEADER + body.len());
        frame.extend_from_slice(&self.next_frame.to_be_bytes());
        frame.extend_from_slice(&self.received.to_be_bytes());
        frame.extend_from_slice(&body);
        (frame, used)
    }

    fn ack_frame(&self) -> Vec<u8> {
        let mut frame = Vec::with_capacity(FRAME_HEADER + 1);
        frame.extend_from_slice(&0u32.to_be_bytes());
        frame.extend_from_slice(&self.received.to_be_bytes());
        frame.push(0);
        frame
    }

    fn take_frame(
        &mut self,
        payload: &[u8],
        now: Instant,
        inbox: &mut Vec<Vec<u8>>,
    ) -> Result<(), FrameFault> {
        if payload.len() < FRAME_HEADER {
            return Err(FrameFault::Broken);
        }
        let frame = read_u32(payload, 0);
        let ack = read_u32(payload, 4);
        let body = unpack(&payload[FRAME_HEADER..], RAW_CHUNK_MAX).ok_or(FrameFault::Broken)?;
        if ack >= self.next_frame || (frame == 0 && !body.is_empty()) {
            return Err(FrameFault::Broken);
        }
        if frame != 0 && frame != self.received.wrapping_add(1) {
            return Err(FrameFault::Lost);
        }
        if ack > self.acked {
            self.acked = ack;
            self.ack_wait = (self.unacked() > 0).then_some(now);
        }
        if frame == 0 {
            return Ok(());
        }
        self.received = frame;
        self.inbound.extend_from_slice(&body);
        let mut start = 0;
        while self.inbound.len() - start >= 4 {
            let len = read_u32(&self.inbound, start) as usize;
            if len > MESSAGE_LIMIT {
                return Err(FrameFault::Lost);
            }
            if self.inbound.len() - start - 4 < len {
                break;
            }
            inbox.push(self.inbound[start + 4..start + 4 + len].to_vec());
            start += 4 + len;
        }
        self.inbound.drain(..start);
        Ok(())
    }
}

#[derive(Resource, Debug)]
pub struct MinecraftWire {
    pub connected: bool,
    pub joined: Vec<u32>,
    pub left: Vec<u32>,
    pub failed: Option<String>,
    pub bulk_failures: Vec<(u32, u32, String)>,
    role: Option<WireRole>,
    budget: usize,
    peers: BTreeMap<u32, Peer>,
    inbox: Vec<(u32, Vec<u8>)>,
    state_inbox: VecDeque<(u32, Vec<u8>)>,
    batches_in: Vec<Vec<u8>>,
    bulk_queue: VecDeque<(u32, u32)>,
    drops: Vec<(u32, String)>,
    last_flush: Option<Instant>,
}

impl Default for MinecraftWire {
    fn default() -> Self {
        Self::new(reliable_budget_from_env())
    }
}

fn reliable_budget_from_env() -> usize {
    std::env::var("IW4L_MC_RELIABLE_KIB")
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|kib| *kib > 0)
        .map_or(RELIABLE_BUDGET, |kib| kib.saturating_mul(1024))
}

impl MinecraftWire {
    pub fn new(budget: usize) -> Self {
        Self {
            connected: false,
            joined: Vec::new(),
            left: Vec::new(),
            failed: None,
            bulk_failures: Vec::new(),
            role: None,
            budget,
            peers: BTreeMap::new(),
            inbox: Vec::new(),
            state_inbox: VecDeque::new(),
            batches_in: Vec::new(),
            bulk_queue: VecDeque::new(),
            drops: Vec::new(),
            last_flush: None,
        }
    }

    pub fn role(&self) -> Option<WireRole> {
        self.role
    }

    pub fn attach(&mut self, role: WireRole) {
        if self.role == Some(role) {
            return;
        }
        *self = Self::new(self.budget);
        self.role = Some(role);
        if role == WireRole::Friend {
            self.peers.insert(HOST, Peer::new());
        }
    }

    pub fn detach(&mut self) {
        if self.role.is_some() {
            *self = Self::new(self.budget);
        }
    }

    fn peer_mut(&mut self, peer: u32) -> Option<&mut Peer> {
        if self.role == Some(WireRole::Friend) && peer != HOST {
            return None;
        }
        self.peers.get_mut(&peer)
    }

    pub fn send(&mut self, to: u32, message: &[u8]) -> bool {
        if message.len() > MESSAGE_LIMIT {
            diag::warn!(
                Net,
                "minecraft message of {} bytes to {to} is over the limit",
                message.len()
            );
            return false;
        }
        let Some(peer) = self.peer_mut(to) else {
            return false;
        };
        peer.queue(message);
        true
    }

    pub fn send_state(&mut self, to: u32, state: &[u8]) -> bool {
        if state.len() > STATE_LIMIT {
            return false;
        }
        let packed = pack(state);
        if packed.len() > STATE_PACKET_BYTES {
            return false;
        }
        let Some(peer) = self.peer_mut(to) else {
            return false;
        };
        peer.state = Some(packed);
        true
    }

    pub fn send_batch(&mut self, to: u32, seq: u32, blob: Arc<Vec<u8>>) -> bool {
        if self.role != Some(WireRole::Host) {
            return false;
        }
        let Some(peer) = self.peers.get_mut(&to) else {
            return false;
        };
        if peer.batches.contains_key(&seq) {
            return true;
        }
        peer.batches.insert(
            seq,
            Batch {
                blob,
                first: None,
                last: None,
                sending: false,
            },
        );
        self.bulk_queue.push_back((to, seq));
        true
    }

    pub fn batch_received(&mut self, from: u32, seq: u32) {
        if let Some(peer) = self.peers.get_mut(&from) {
            peer.batches.remove(&seq);
        }
        self.bulk_queue.retain(|queued| *queued != (from, seq));
    }

    pub fn batches_in_flight(&self) -> usize {
        self.peers
            .values()
            .flat_map(|peer| peer.batches.values())
            .filter(|batch| batch.first.is_some())
            .count()
    }

    pub fn unacked(&self, peer: u32) -> Option<u32> {
        self.peers.get(&peer).map(Peer::unacked)
    }

    pub fn backlog(&self, peer: u32) -> Option<usize> {
        self.peers.get(&peer).map(|peer| peer.out.len())
    }

    pub fn take_inbox(&mut self) -> Vec<(u32, Vec<u8>)> {
        std::mem::take(&mut self.inbox)
    }

    pub fn take_state(&mut self) -> Vec<(u32, Vec<u8>)> {
        self.state_inbox.drain(..).collect()
    }

    pub fn take_batches(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.batches_in)
    }

    pub fn take_drops(&mut self) -> Vec<(u32, String)> {
        std::mem::take(&mut self.drops)
    }

    pub fn receive_batch(&mut self, blob: Vec<u8>) {
        if self.role == Some(WireRole::Friend) && self.failed.is_none() {
            self.batches_in.push(blob);
        }
    }

    pub fn receive(&mut self, from: u32, lane: MinecraftLane, payload: Vec<u8>, now: Instant) {
        let Some(role) = self.role else {
            return;
        };
        if self.failed.is_some() || (role == WireRole::Friend && from != HOST) {
            return;
        }
        if role == WireRole::Host && !self.peers.contains_key(&from) {
            self.peers.insert(from, Peer::new());
            self.joined.push(from);
        }
        match lane {
            MinecraftLane::State => match unpack(&payload, STATE_LIMIT) {
                Some(state) => {
                    self.state_inbox.push_back((from, state));
                    while self.state_inbox.len() > STATE_INBOX {
                        self.state_inbox.pop_front();
                    }
                }
                None => self.bad_message(from, now),
            },
            MinecraftLane::Reliable => {
                let Some(peer) = self.peers.get_mut(&from) else {
                    return;
                };
                let mut messages = Vec::new();
                let outcome = peer.take_frame(&payload, now, &mut messages);
                self.inbox
                    .extend(messages.into_iter().map(|message| (from, message)));
                match outcome {
                    Ok(()) => {}
                    Err(FrameFault::Broken) => self.bad_message(from, now),
                    Err(FrameFault::Lost) => self.drop_peer(from, LOST_STREAM),
                }
            }
        }
    }

    pub fn bad_message(&mut self, from: u32, now: Instant) {
        let Some(peer) = self.peers.get_mut(&from) else {
            return;
        };
        diag::warn!(Net, "minecraft dropped a broken message from {from}");
        peer.bad.push_back(now);
        while peer
            .bad
            .front()
            .is_some_and(|at| now.duration_since(*at) > BAD_WINDOW)
        {
            peer.bad.pop_front();
        }
        if self.role == Some(WireRole::Host) && peer.bad.len() >= BAD_LIMIT {
            self.drop_peer(from, BROKEN_MESSAGES);
        }
    }

    pub fn drop_peer(&mut self, peer: u32, reason: &str) {
        match self.role {
            Some(WireRole::Host) => {
                if self.peers.remove(&peer).is_none() {
                    return;
                }
                diag::warn!(Net, "minecraft client {peer} dropped: {reason}");
                self.bulk_queue.retain(|(id, _)| *id != peer);
                self.left.push(peer);
                self.drops.push((peer, reason.to_owned()));
            }
            Some(WireRole::Friend) => {
                if self.failed.is_none() {
                    diag::warn!(Net, "minecraft link to the host failed: {reason}");
                    self.failed = Some(reason.to_owned());
                }
            }
            None => {}
        }
    }

    pub fn retired(&mut self, peer: u32) {
        if self.role == Some(WireRole::Host) && self.peers.remove(&peer).is_some() {
            self.bulk_queue.retain(|(id, _)| *id != peer);
            self.left.push(peer);
        }
    }

    pub fn note_bulk_report(
        &mut self,
        peer: u32,
        seq: u32,
        result: Result<(), String>,
        now: Instant,
    ) {
        let Some(batches) = self.peers.get_mut(&peer).map(|peer| &mut peer.batches) else {
            return;
        };
        if let Some(batch) = batches.get_mut(&seq) {
            batch.sending = false;
            batch.last = Some(now);
        }
        if let Err(error) = result {
            self.bulk_failures.push((peer, seq, error));
            if self.bulk_failures.len() > BULK_FAILURES {
                self.bulk_failures.remove(0);
            }
        }
    }

    pub fn pump(&mut self, now: Instant, link: &mut impl MinecraftLink) {
        let Some(role) = self.role else {
            return;
        };
        if !self.connected || self.failed.is_some() {
            return;
        }
        self.check_timers(now);
        for (id, peer) in &mut self.peers {
            if let Some(state) = peer.state.take() {
                link.send_minecraft_to(*id, MinecraftLane::State, state);
            }
        }
        if role == WireRole::Host {
            self.pump_bulk(now, link);
        }
        if self
            .last_flush
            .is_some_and(|last| now.duration_since(last) < TICK)
        {
            return;
        }
        self.last_flush = Some(now);
        self.pump_reliable(role, now, link);
    }

    fn check_timers(&mut self, now: Instant) {
        let mut drops = Vec::new();
        for (id, peer) in &self.peers {
            let waited = peer
                .ack_wait
                .is_some_and(|since| now.duration_since(since) >= ACK_TIMEOUT);
            if waited || peer.out.len() > BACKLOG_LIMIT {
                drops.push((*id, TOO_SLOW));
            } else if peer.batches.values().any(|batch| {
                batch
                    .first
                    .is_some_and(|first| now.duration_since(first) >= BATCH_GIVE_UP)
            }) {
                drops.push((*id, LOST_STREAM));
            }
        }
        for (id, reason) in drops {
            self.drop_peer(id, reason);
        }
    }

    fn pump_bulk(&mut self, now: Instant, link: &mut impl MinecraftLink) {
        let mut in_flight = 0;
        for (id, peer) in &mut self.peers {
            for (seq, batch) in &mut peer.batches {
                let Some(last) = batch.last else {
                    continue;
                };
                in_flight += 1;
                if !batch.sending && now.duration_since(last) >= BATCH_RESEND {
                    diag::info!(Net, "minecraft batch {seq} to client {id} sent again");
                    batch.sending = link.send_bulk_to(*id, *seq, Arc::clone(&batch.blob));
                    batch.last = Some(now);
                }
            }
        }
        while in_flight < BULK_STREAMS {
            let Some((id, seq)) = self.bulk_queue.pop_front() else {
                break;
            };
            let Some(batch) = self
                .peers
                .get_mut(&id)
                .and_then(|peer| peer.batches.get_mut(&seq))
            else {
                continue;
            };
            if batch.first.is_some() {
                continue;
            }
            batch.sending = link.send_bulk_to(id, seq, Arc::clone(&batch.blob));
            batch.first = Some(now);
            batch.last = Some(now);
            in_flight += 1;
        }
    }

    fn pump_reliable(&mut self, role: WireRole, now: Instant, link: &mut impl MinecraftLink) {
        let (window, per_flush, budget) = match role {
            WireRole::Host => {
                let friends = (self.peers.len() as u32).max(1);
                let window = HOST_WINDOW.min((WINDOW_TOTAL / friends).max(1));
                (window, usize::MAX, Some(self.budget))
            }
            WireRole::Friend => (FRIEND_WINDOW, 1, None),
        };
        let mut room = MAILBOX_LIMIT.saturating_sub(link.control_queued());
        for (id, peer) in &mut self.peers {
            let mut sent = 0;
            while room > 0 && sent < per_flush && peer.unacked() < window && !peer.out.is_empty() {
                let left = budget.map(|budget| peer.budget_left(now, budget));
                if left == Some(0)
                    || left.is_some_and(|left| {
                        peer.budget_used > 0
                            && left < FRAME_BYTES
                            && peer.out.len() + FRAME_HEADER + PACK_HEADER > left
                    })
                {
                    break;
                }
                let (frame, used) = peer.data_frame();
                let len = frame.len();
                if left.is_some_and(|left| len > left) && peer.budget_used > 0 {
                    break;
                }
                if !link.send_minecraft_to(*id, MinecraftLane::Reliable, frame) {
                    room = 0;
                    break;
                }
                peer.out.drain(..used);
                peer.next_frame += 1;
                peer.ack_sent = peer.received;
                peer.budget_used += len;
                if peer.ack_wait.is_none() {
                    peer.ack_wait = Some(now);
                }
                room -= 1;
                sent += 1;
            }
            if sent == 0
                && room > 0
                && peer.received != peer.ack_sent
                && link.send_minecraft_to(*id, MinecraftLane::Reliable, peer.ack_frame())
            {
                peer.ack_sent = peer.received;
                room -= 1;
            }
        }
    }
}

pub(crate) fn register(app: &mut App) {
    app.init_resource::<MinecraftWire>()
        .add_systems(PreUpdate, receive_minecraft)
        .add_systems(PostUpdate, send_minecraft);
}

fn receive_minecraft(
    bridge: Option<Res<MasterBridge>>,
    hub: Option<ResMut<UdpAuthorityHub>>,
    link: Option<ResMut<UdpClientLink>>,
    mut wire: ResMut<MinecraftWire>,
) {
    let now = Instant::now();
    if let Some(mut hub) = hub {
        wire.attach(WireRole::Host);
        wire.connected = true;
        for client in hub.take_retired() {
            wire.retired(client.0);
        }
        for (client, lane, payload) in hub.take_minecraft() {
            wire.receive(client.0, lane, payload, now);
        }
        for report in hub.mailbox().minecraft_bulk().take_reports() {
            if let Some(client) = hub.client_of_member(report.member) {
                wire.note_bulk_report(client.0, report.seq, report.result, now);
            }
        }
        return;
    }
    if let Some(mut link) = link {
        wire.attach(WireRole::Friend);
        wire.connected =
            bridge.is_some_and(|bridge| matches!(bridge.state(), MasterBridgeState::Joined { .. }));
        for (lane, payload) in link.take_minecraft() {
            wire.receive(HOST, lane, payload, now);
        }
        for blob in link.mailbox().minecraft_bulk().take_inbound() {
            wire.receive_batch(blob);
        }
        return;
    }
    wire.detach();
}

fn send_minecraft(
    hub: Option<ResMut<UdpAuthorityHub>>,
    link: Option<ResMut<UdpClientLink>>,
    mut authority: Option<ResMut<AuthorityWorld>>,
    mut wire: ResMut<MinecraftWire>,
) {
    let now = Instant::now();
    if let Some(mut hub) = hub {
        wire.pump(now, &mut *hub);
        for (client, reason) in wire.take_drops() {
            hub.retire_client_with_reason(ClientId(client), &reason);
            if let Some(authority) = authority.as_mut() {
                authority.0.retire_client(ClientId(client));
            }
        }
        return;
    }
    if let Some(mut link) = link {
        wire.pump(now, &mut *link);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authority::inbox::{ClientActionInbox, ClientCommandInbox};
    use crate::transport::bootstrap::BootstrapLane;
    use crate::transport::master::route_uni_stream;
    use crate::transport::protocol::{
        ClientPacket, ContentFingerprint, HandshakeHello, ProtocolLimits, ServerPacket,
        decode_client_packet, decode_server_packet,
    };
    use crate::transport::udp_session::RelayMailbox;

    const LOCAL: MemberId = MemberId([1; 16]);
    const FRIEND: MemberId = MemberId([7; 16]);

    fn hello() -> HandshakeHello {
        HandshakeHello::current(ContentFingerprint::default())
    }

    fn noise(len: usize, seed: u64) -> Vec<u8> {
        let mut state = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state as u8
            })
            .collect()
    }

    fn ack_only(ack: u32) -> Vec<u8> {
        let mut frame = 0u32.to_be_bytes().to_vec();
        frame.extend_from_slice(&ack.to_be_bytes());
        frame.push(0);
        frame
    }

    fn host_with(peers: &[u32], budget: usize, now: Instant) -> MinecraftWire {
        let mut wire = MinecraftWire::new(budget);
        wire.attach(WireRole::Host);
        wire.connected = true;
        for peer in peers {
            wire.receive(*peer, MinecraftLane::Reliable, ack_only(0), now);
        }
        assert_eq!(wire.joined, peers);
        wire.joined.clear();
        wire
    }

    fn friend_wire() -> MinecraftWire {
        let mut wire = MinecraftWire::new(RELIABLE_BUDGET);
        wire.attach(WireRole::Friend);
        wire.connected = true;
        wire
    }

    #[derive(Default)]
    struct TestLink {
        queued: usize,
        reliable: Vec<(u32, Vec<u8>)>,
        state: Vec<(u32, Vec<u8>)>,
        bulk: Vec<(u32, u32, Arc<Vec<u8>>)>,
    }

    impl MinecraftLink for TestLink {
        fn control_queued(&self) -> usize {
            self.queued
        }

        fn send_minecraft_to(&mut self, peer: u32, lane: MinecraftLane, payload: Vec<u8>) -> bool {
            match lane {
                MinecraftLane::Reliable => {
                    assert!(payload.len() <= FRAME_BYTES);
                    self.reliable.push((peer, payload));
                }
                MinecraftLane::State => self.state.push((peer, payload)),
            }
            true
        }

        fn send_bulk_to(&mut self, peer: u32, seq: u32, blob: Arc<Vec<u8>>) -> bool {
            self.bulk.push((peer, seq, blob));
            true
        }
    }

    fn exchange(
        host: &mut MinecraftWire,
        friend: &mut MinecraftWire,
        now: Instant,
    ) -> (usize, usize) {
        let mut to_friend = TestLink::default();
        host.pump(now, &mut to_friend);
        let bytes = to_friend
            .reliable
            .iter()
            .map(|(_, frame)| frame.len())
            .sum();
        for (_, frame) in to_friend.reliable {
            friend.receive(HOST, MinecraftLane::Reliable, frame, now);
        }
        for (_, state) in to_friend.state {
            friend.receive(HOST, MinecraftLane::State, state, now);
        }
        let mut to_host = TestLink::default();
        friend.pump(now, &mut to_host);
        let frames = to_host.reliable.len();
        for (_, frame) in to_host.reliable {
            host.receive(1, MinecraftLane::Reliable, frame, now);
        }
        for (_, state) in to_host.state {
            host.receive(1, MinecraftLane::State, state, now);
        }
        (bytes, frames)
    }

    #[test]
    fn minecraft_packets_round_trip_with_their_own_tags() {
        let limits = ProtocolLimits::default_listen();
        let payload = vec![1, 2, 3, 250];
        for (packet, tag) in [
            (ClientPacket::Minecraft(payload.clone()), 4),
            (ClientPacket::MinecraftState(payload.clone()), 5),
        ] {
            let bytes = packet.to_bytes();
            assert_eq!(bytes[0], tag);
            assert_eq!(decode_client_packet(&bytes, &limits), Ok(packet));
        }
        for (packet, tag) in [
            (ServerPacket::Minecraft(payload.clone()), 15),
            (ServerPacket::MinecraftState(payload.clone()), 16),
        ] {
            let bytes = packet.to_bytes();
            assert_eq!(bytes[0], tag);
            assert_eq!(decode_server_packet(&bytes, &limits), Ok(packet));
        }
        let good = ServerPacket::Minecraft(payload).to_bytes();
        let mut lying = good.clone();
        lying[1..5].copy_from_slice(&[0xff; 4]);
        assert!(decode_server_packet(&lying, &limits).is_err());
        assert!(decode_server_packet(&good[..good.len() - 1], &limits).is_err());
        let mut trailing = good;
        trailing.push(0);
        assert!(decode_server_packet(&trailing, &limits).is_err());
    }

    #[test]
    fn a_broken_minecraft_packet_does_not_end_the_link() {
        let mailbox = RelayMailbox::new(64);
        let mut link = UdpClientLink::relay(hello(), mailbox.clone());
        let mut broken = ServerPacket::Minecraft(vec![9; 8]).to_bytes();
        broken.truncate(broken.len() - 3);
        for packet in [
            broken,
            ServerPacket::Minecraft(b"junk".to_vec()).to_bytes(),
            ServerPacket::Minecraft(b"after".to_vec()).to_bytes(),
        ] {
            mailbox.push_control_inbound(LOCAL, packet).unwrap();
        }
        mailbox
            .push_inbound(
                LOCAL,
                ServerPacket::MinecraftState(b"state".to_vec()).to_bytes(),
            )
            .unwrap();
        assert!(link.recv_ticks().is_ok());
        let received = link.take_minecraft();
        assert_eq!(
            received,
            vec![
                (MinecraftLane::State, b"state".to_vec()),
                (MinecraftLane::Reliable, b"junk".to_vec()),
                (MinecraftLane::Reliable, b"after".to_vec()),
            ]
        );
        let mut wire = friend_wire();
        let now = Instant::now();
        for (lane, payload) in received {
            wire.receive(HOST, lane, payload, now);
        }
        assert_eq!(wire.failed, None);
        assert!(wire.take_inbox().is_empty());
    }

    #[test]
    fn minecraft_from_an_enrolled_but_uncommitted_peer_is_delivered() {
        let mailbox = RelayMailbox::new(64);
        let mut hub = UdpAuthorityHub::relay(hello(), 1, mailbox.clone());
        hub.attach_bootstrap(BootstrapLane::new());
        hub.reconcile_relay_membership(&[LOCAL, FRIEND], LOCAL);
        let stranger = MemberId([9; 16]);
        for (member, packet) in [
            (FRIEND, ClientPacket::Minecraft(b"hello".to_vec())),
            (FRIEND, ClientPacket::MinecraftState(b"wrong lane".to_vec())),
            (stranger, ClientPacket::Minecraft(b"stranger".to_vec())),
        ] {
            mailbox
                .push_control_inbound(member, packet.to_bytes())
                .unwrap();
        }
        for (member, packet) in [
            (FRIEND, ClientPacket::MinecraftState(b"held".to_vec())),
            (FRIEND, ClientPacket::Minecraft(b"wrong lane".to_vec())),
        ] {
            mailbox.push_inbound(member, packet.to_bytes()).unwrap();
        }
        hub.ingress(
            &mut ClientCommandInbox::default(),
            &mut ClientActionInbox::default(),
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            hub.take_minecraft(),
            vec![
                (ClientId(1), MinecraftLane::State, b"held".to_vec()),
                (ClientId(1), MinecraftLane::Reliable, b"hello".to_vec()),
            ]
        );
        assert!(
            hub.send_minecraft(ClientId(1), MinecraftLane::Reliable, b"hi".to_vec())
                .is_ok()
        );
        assert_eq!(
            mailbox.take_control_outbound(),
            vec![(FRIEND, ServerPacket::Minecraft(b"hi".to_vec()).to_bytes())]
        );
        hub.retire_member(FRIEND);
        assert_eq!(hub.take_retired(), vec![ClientId(1)]);
        let mut wire = host_with(&[1], RELIABLE_BUDGET, Instant::now());
        wire.retired(1);
        assert_eq!(wire.left, vec![1]);
        assert!(wire.take_drops().is_empty());
    }

    #[test]
    fn a_friend_that_never_acknowledges_gets_only_the_window() {
        let mailbox = RelayMailbox::new(64);
        let mut hub = UdpAuthorityHub::relay(hello(), 1, mailbox.clone());
        hub.reconcile_relay_membership(&[LOCAL, FRIEND], LOCAL);
        let start = Instant::now();
        let mut wire = host_with(&[1], usize::MAX, start);
        assert!(wire.send(1, &noise(1 << 20, 1)));
        let mut sent = 0;
        for tick in 1..=100 {
            wire.pump(start + TICK * tick, &mut hub);
            sent += mailbox.take_control_outbound().len();
        }
        assert_eq!(sent, HOST_WINDOW as usize);
        assert_eq!(wire.unacked(1), Some(HOST_WINDOW));

        let mut link = TestLink::default();
        let mut four = host_with(&[1, 2, 3, 4], usize::MAX, start);
        for peer in 1..=4 {
            four.send(peer, &noise(1 << 20, u64::from(peer)));
        }
        for tick in 1..=100 {
            four.pump(start + TICK * tick, &mut link);
        }
        for peer in 1..=4 {
            assert_eq!(four.unacked(peer), Some(WINDOW_TOTAL / 4));
        }
    }

    #[test]
    fn minecraft_frames_leave_room_in_the_mailbox_for_mw2() {
        let mailbox = RelayMailbox::new(64);
        let mut hub = UdpAuthorityHub::relay(hello(), 1, mailbox.clone());
        let members = [MemberId([2; 16]), MemberId([3; 16]), MemberId([4; 16])];
        hub.reconcile_relay_membership(&members, LOCAL);
        for _ in 0..30 {
            mailbox
                .push_control_outbound(members[0], b"mw2".to_vec())
                .unwrap();
        }
        let start = Instant::now();
        let mut wire = host_with(&[1, 2, 3], usize::MAX, start);
        for peer in 1..=3 {
            wire.send(peer, &noise(1 << 20, u64::from(peer)));
        }
        for tick in 1..=40 {
            wire.pump(start + TICK * tick, &mut hub);
            assert!(mailbox.control_outbound_len() <= MAILBOX_LIMIT);
        }
        assert_eq!(mailbox.control_outbound_len(), MAILBOX_LIMIT);
        mailbox.take_control_outbound();
        for tick in 41..=80 {
            wire.pump(start + TICK * tick, &mut hub);
            mailbox.take_control_outbound();
        }
        for peer in 1..=3 {
            assert_eq!(wire.unacked(peer), Some(HOST_WINDOW));
        }
    }

    #[test]
    fn the_byte_budget_spreads_a_backlog_over_seconds() {
        let start = Instant::now();
        let mut host = host_with(&[1], RELIABLE_BUDGET, start);
        let mut friend = friend_wire();
        let messages: Vec<Vec<u8>> = (0..64).map(|seed| noise(16 * 1024, seed)).collect();
        for message in &messages {
            assert!(host.send(1, message));
        }
        let mut first_second = 0;
        let mut done = None;
        for tick in 1..=600 {
            let (bytes, _) = exchange(&mut host, &mut friend, start + TICK * tick);
            if tick <= 20 {
                first_second += bytes;
            }
            if done.is_none() && host.backlog(1) == Some(0) {
                done = Some(tick);
            }
        }
        assert!(first_second <= RELIABLE_BUDGET, "{first_second}");
        let done = done.expect("the backlog drains");
        assert!((15 * 20..=20 * 20).contains(&done), "{done}");
        let received: Vec<Vec<u8>> = friend
            .take_inbox()
            .into_iter()
            .map(|(_, message)| message)
            .collect();
        assert_eq!(received, messages);
    }

    #[test]
    fn messages_of_any_size_cross_both_ways_in_order() {
        let start = Instant::now();
        let mut host = host_with(&[1], RELIABLE_BUDGET * 16, start);
        let mut friend = friend_wire();
        let to_friend = vec![
            b"login".to_vec(),
            noise(100 * 1024, 3),
            vec![0; 300 * 1024],
            Vec::new(),
            b"chat".to_vec(),
        ];
        let mut to_host: Vec<Vec<u8>> = (0..10u8).map(|n| vec![n; 10]).collect();
        to_host.push(noise(40 * 1024, 4));
        for message in &to_friend {
            assert!(host.send(1, message));
        }
        for message in &to_host {
            assert!(friend.send(HOST, message));
        }
        assert!(!friend.send(5, b"not the host"));
        assert!(host.send_state(1, &[5; 3000]));
        assert!(friend.send_state(HOST, b"held"));
        for tick in 1..=200 {
            let (_, frames) = exchange(&mut host, &mut friend, start + TICK * tick);
            assert!(frames <= 1);
            assert!(
                friend
                    .unacked(HOST)
                    .is_some_and(|unacked| unacked <= FRIEND_WINDOW)
            );
        }
        let at_friend: Vec<Vec<u8>> = friend
            .take_inbox()
            .into_iter()
            .map(|(_, message)| message)
            .collect();
        assert_eq!(at_friend, to_friend);
        let expected: Vec<(u32, Vec<u8>)> =
            to_host.into_iter().map(|message| (1, message)).collect();
        assert_eq!(host.take_inbox(), expected);
        assert_eq!(friend.take_state(), vec![(HOST, vec![5; 3000])]);
        assert_eq!(host.take_state(), vec![(1, b"held".to_vec())]);
        assert_eq!(host.unacked(1), Some(0));
        assert_eq!(friend.unacked(HOST), Some(0));
    }

    #[test]
    fn nothing_is_sent_before_the_link_is_up() {
        let mut friend = MinecraftWire::new(RELIABLE_BUDGET);
        friend.attach(WireRole::Friend);
        assert!(friend.send(HOST, b"hello"));
        let mut link = TestLink::default();
        friend.pump(Instant::now(), &mut link);
        assert!(link.reliable.is_empty());
        friend.connected = true;
        friend.pump(Instant::now(), &mut link);
        assert_eq!(link.reliable.len(), 1);
    }

    #[test]
    fn an_unacknowledged_batch_is_sent_again_with_the_same_seq() {
        let start = Instant::now();
        let mut wire = host_with(&[1, 2], RELIABLE_BUDGET, start);
        let blobs: Vec<Arc<Vec<u8>>> = (0..3u8).map(|n| Arc::new(vec![n; 32])).collect();
        assert!(wire.send_batch(1, 7, Arc::clone(&blobs[0])));
        assert!(wire.send_batch(1, 8, Arc::clone(&blobs[1])));
        assert!(wire.send_batch(2, 1, Arc::clone(&blobs[2])));
        let sent = |link: &mut TestLink| -> Vec<(u32, u32)> {
            link.bulk
                .drain(..)
                .map(|(peer, seq, _)| (peer, seq))
                .collect()
        };
        let mut link = TestLink::default();
        wire.pump(start, &mut link);
        assert!(Arc::ptr_eq(&link.bulk[0].2, &blobs[0]));
        assert_eq!(sent(&mut link), vec![(1, 7), (1, 8)]);
        assert_eq!(wire.batches_in_flight(), BULK_STREAMS);
        wire.pump(start + BATCH_RESEND * 2, &mut link);
        assert!(sent(&mut link).is_empty());
        let reported = start + BATCH_RESEND * 2 + TICK;
        wire.note_bulk_report(1, 7, Ok(()), reported);
        wire.note_bulk_report(1, 8, Err("refused".to_owned()), reported);
        assert_eq!(wire.bulk_failures, vec![(1, 8, "refused".to_owned())]);
        wire.pump(reported + Duration::from_millis(2900), &mut link);
        assert!(sent(&mut link).is_empty());
        wire.pump(reported + BATCH_RESEND, &mut link);
        assert!(Arc::ptr_eq(&link.bulk[0].2, &blobs[0]));
        assert_eq!(sent(&mut link), vec![(1, 7), (1, 8)]);
        wire.batch_received(1, 7);
        wire.pump(reported + BATCH_RESEND + TICK, &mut link);
        assert_eq!(sent(&mut link), vec![(2, 1)]);
        wire.pump(start + BATCH_GIVE_UP, &mut link);
        assert_eq!(wire.take_drops(), vec![(1, LOST_STREAM.to_owned())]);
        assert_eq!(wire.left, vec![1]);
        assert!(wire.unacked(2).is_some());
    }

    #[derive(Clone)]
    struct FlakySender;

    impl BulkSender for FlakySender {
        async fn send_bulk(&self, _member: MemberId, blob: Arc<Vec<u8>>) -> Result<(), String> {
            if blob.as_slice() == b"bad" {
                Err("stream reset".to_owned())
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn a_failed_minecraft_write_is_reported_and_the_link_stays_up() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        let bulk = Arc::new(MinecraftBulk::default());
        let cancel = CancellationToken::new();
        let mut reports = runtime.block_on(async {
            let egress = tokio::spawn(minecraft_egress(
                FlakySender,
                Arc::clone(&bulk),
                cancel.clone(),
            ));
            let mut reports = Vec::new();
            for (seq, blob) in [(1, b"bad".to_vec()), (2, b"good".to_vec())] {
                bulk.push_outbound(FRIEND, seq, Arc::new(blob)).unwrap();
                for _ in 0..200 {
                    reports.extend(bulk.take_reports());
                    if reports.len() == seq as usize {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                assert!(!egress.is_finished());
            }
            cancel.cancel();
            assert_eq!(egress.await.unwrap(), Ok(()));
            reports
        });
        reports.sort_by_key(|report| report.seq);
        assert_eq!(
            reports,
            vec![
                BulkReport {
                    member: FRIEND,
                    seq: 1,
                    result: Err("stream reset".to_owned()),
                },
                BulkReport {
                    member: FRIEND,
                    seq: 2,
                    result: Ok(()),
                },
            ]
        );
        let mut wire = host_with(&[1], RELIABLE_BUDGET, Instant::now());
        for report in reports {
            wire.note_bulk_report(1, report.seq, report.result, Instant::now());
        }
        assert_eq!(wire.bulk_failures, vec![(1, 1, "stream reset".to_owned())]);
        assert!(wire.take_drops().is_empty());
        assert!(wire.unacked(1).is_some());
    }

    struct FakeStream {
        data: Vec<u8>,
        pos: usize,
        cut: Option<usize>,
    }

    impl FakeStream {
        fn new(payload: &[u8], cut: Option<usize>) -> Self {
            Self {
                data: encode_relay_stream(RelayDatagram::ServiceToMember(payload)).unwrap(),
                pos: 0,
                cut,
            }
        }
    }

    impl UniStream for FakeStream {
        async fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), String> {
            let end = self.pos + buf.len();
            if end > self.cut.unwrap_or(self.data.len()) {
                return Err("stream reset".to_owned());
            }
            buf.copy_from_slice(&self.data[self.pos..end]);
            self.pos = end;
            Ok(())
        }

        async fn read_to_end(&mut self, limit: usize) -> Result<Vec<u8>, String> {
            if self.cut.is_some() {
                return Err("stream reset".to_owned());
            }
            let rest = self.data[self.pos..].to_vec();
            if rest.len() > limit {
                return Err("too long".to_owned());
            }
            self.pos = self.data.len();
            Ok(rest)
        }
    }

    #[test]
    fn a_broken_minecraft_stream_is_dropped_and_bootstrap_still_arrives() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        let lane = BootstrapLane::new();
        let bulk = Arc::new(MinecraftBulk::default());
        let batch = b"MC\x01batch".to_vec();
        runtime.block_on(async {
            let mut readers = tokio::task::JoinSet::new();
            for stream in [
                FakeStream::new(&batch, Some(8)),
                FakeStream::new(b"BS-offer", None),
                FakeStream::new(&batch, None),
            ] {
                route_uni_stream(stream, &lane, &bulk, &mut readers, "join")
                    .await
                    .unwrap();
            }
            while readers.join_next().await.is_some() {}
        });
        let offers: Vec<Vec<u8>> = lane
            .take_from_worker()
            .into_iter()
            .map(|ingress| ingress.bytes)
            .collect();
        assert_eq!(offers, vec![b"BS-offer".to_vec()]);
        assert_eq!(bulk.take_inbound(), vec![batch]);
    }

    #[test]
    fn a_silent_or_flooded_friend_is_dropped_as_too_slow() {
        let start = Instant::now();
        let mut wire = host_with(&[1, 2], RELIABLE_BUDGET, start);
        wire.send(1, b"hello");
        let mut link = TestLink::default();
        wire.pump(start + TICK, &mut link);
        assert_eq!(wire.unacked(1), Some(1));
        wire.pump(start + ACK_TIMEOUT, &mut link);
        assert!(wire.take_drops().is_empty());
        wire.send(2, &vec![0; 3 << 20]);
        wire.send(2, &vec![0; 3 << 20]);
        wire.pump(start + TICK + ACK_TIMEOUT, &mut link);
        let mut drops = wire.take_drops();
        drops.sort();
        assert_eq!(
            drops,
            vec![(1, TOO_SLOW.to_owned()), (2, TOO_SLOW.to_owned())]
        );
        assert_eq!(wire.left.len(), 2);
    }

    #[test]
    fn broken_frames_count_and_a_gap_loses_the_stream() {
        let start = Instant::now();
        let mut wire = host_with(&[1, 2], RELIABLE_BUDGET, start);
        for _ in 0..BAD_LIMIT - 1 {
            wire.receive(1, MinecraftLane::Reliable, vec![1, 2, 3], start);
        }
        assert!(wire.take_drops().is_empty());
        wire.receive(1, MinecraftLane::Reliable, vec![1, 2, 3], start);
        assert_eq!(wire.take_drops(), vec![(1, BROKEN_MESSAGES.to_owned())]);
        let mut gap = 5u32.to_be_bytes().to_vec();
        gap.extend_from_slice(&0u32.to_be_bytes());
        gap.push(0);
        wire.receive(2, MinecraftLane::Reliable, gap, start);
        assert_eq!(wire.take_drops(), vec![(2, LOST_STREAM.to_owned())]);
        let mut friend = friend_wire();
        for _ in 0..BAD_LIMIT * 2 {
            friend.receive(HOST, MinecraftLane::Reliable, vec![1], start);
        }
        assert_eq!(friend.failed, None);
    }
}
