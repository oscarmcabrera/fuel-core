use crate::config::fuel_upgrade::Checksum;
use futures::{
    future,
    AsyncRead,
    AsyncWrite,
    Future,
    TryFutureExt,
};
use libp2p::{
    core::{
        upgrade::{
            InboundConnectionUpgrade,
            OutboundConnectionUpgrade,
        },
        UpgradeInfo,
    },
    noise::{
        Config as NoiseConfig,
        Error as NoiseError,
        Output as NoiseOutput,
    },
    PeerId,
};
use std::pin::Pin;

pub(crate) trait Approver {
    /// Allows Peer connection based on it's PeerId and the Approver's knowledge of the Connection State
    fn allow_peer(&self, peer_id: &PeerId) -> bool;
}

#[derive(Clone)]
pub(crate) struct FuelAuthenticated<A: Approver> {
    noise_authenticated: NoiseConfig,
    approver: A,
    checksum: Checksum,
}

impl<A: Approver> FuelAuthenticated<A> {
    pub(crate) fn new(
        noise_authenticated: NoiseConfig,
        approver: A,
        checksum: Checksum,
    ) -> Self {
        Self {
            noise_authenticated,
            approver,
            checksum,
        }
    }
}

impl<A: Approver> UpgradeInfo for FuelAuthenticated<A> {
    type Info = String;
    type InfoIter = std::iter::Once<String>;

    fn protocol_info(&self) -> Self::InfoIter {
        let noise = self
            .noise_authenticated
            .protocol_info()
            .next()
            .expect("Noise always has a protocol info");

        std::iter::once(format!("{}/{}", noise, hex::encode(self.checksum.as_ref())))
    }
}

impl<A, T> InboundConnectionUpgrade<T> for FuelAuthenticated<A>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    A: Approver + Send + 'static,
{
    type Output = (PeerId, NoiseOutput<T>);
    type Error = NoiseError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_inbound(self, socket: T, _: Self::Info) -> Self::Future {
        Box::pin(
            self.noise_authenticated
                .upgrade_inbound(socket, "")
                .and_then(move |(remote_peer_id, io)| {
                    if self.approver.allow_peer(&remote_peer_id) {
                        future::ok((remote_peer_id, io))
                    } else {
                        future::err(NoiseError::AuthenticationFailed)
                    }
                }),
        )
    }
}

impl<A, T> OutboundConnectionUpgrade<T> for FuelAuthenticated<A>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    A: Approver + Send + 'static,
{
    type Output = (PeerId, NoiseOutput<T>);
    type Error = NoiseError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_outbound(self, socket: T, _: Self::Info) -> Self::Future {
        Box::pin(
            self.noise_authenticated
                .upgrade_outbound(socket, "")
                .and_then(move |(remote_peer_id, io)| {
                    if self.approver.allow_peer(&remote_peer_id) {
                        future::ok((remote_peer_id, io))
                    } else {
                        future::err(NoiseError::AuthenticationFailed)
                    }
                }),
        )
    }
}
