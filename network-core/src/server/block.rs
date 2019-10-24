//! Block service abstraction.

use super::P2pService;
use crate::{error::Error, subscription::BlockEvent};

use chain_core::property::{Block, BlockDate, BlockId, HasHeader, Header};

use futures::prelude::*;

/// Interface for the blockchain node service implementation responsible for
/// providing access to block data.
pub trait BlockService: P2pService {
    /// The block identifier type for the blockchain.
    type BlockId: BlockId;

    /// The block date type for the blockchain.
    type BlockDate: BlockDate;

    /// The type representing a block on the blockchain.
    type Block: Block<Id = Self::BlockId, Date = Self::BlockDate> + HasHeader<Header = Self::Header>;

    /// The type representing metadata header of a block.
    type Header: Header<Id = Self::BlockId, Date = Self::BlockDate>;

    /// The type of asynchronous futures returned by method `tip`.
    ///
    /// The future resolves to the block identifier and the block date
    /// of the current chain tip as known by the serving node.
    type TipFuture: Future<Item = Self::Header, Error = Error> + Send + 'static;

    /// The type of an asynchronous stream that provides blocks in
    /// response to `pull_blocks*` methods.
    type PullBlocksStream: Stream<Item = Self::Block, Error = Error> + Send + 'static;

    /// The type of asynchronous futures returned by `pull_blocks` method.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type PullBlocksFuture: Future<Item = Self::PullBlocksStream, Error = Error> + Send + 'static;

    /// The type of asynchronous futures returned by `pull_blocks_to_tip` method.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type PullBlocksToTipFuture: Future<Item = Self::PullBlocksStream, Error = Error>
        + Send
        + 'static;

    /// The type of an asynchronous stream that provides blocks in
    /// response to `get_blocks` method.
    type GetBlocksStream: Stream<Item = Self::Block, Error = Error> + Send + 'static;

    /// The type of asynchronous futures returned by `get_blocks` methods.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GetBlocksFuture: Future<Item = Self::GetBlocksStream, Error = Error> + Send + 'static;

    /// The type of an asynchronous stream that provides block headers in
    /// response to `pull_headers*` methods.
    type PullHeadersStream: Stream<Item = Self::Header, Error = Error> + Send + 'static;

    /// The type of asynchronous futures returned by `pull_headers` method.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type PullHeadersFuture: Future<Item = Self::PullHeadersStream, Error = Error> + Send + 'static;

    /// The type of an asynchronous stream that provides block headers in
    /// response to `get_headers` methods.
    type GetHeadersStream: Stream<Item = Self::Header, Error = Error> + Send + 'static;

    /// The type of asynchronous futures returned by `get_headers` methods.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GetHeadersFuture: Future<Item = Self::GetHeadersStream, Error = Error> + Send + 'static;

    /// The application-provided sink object that receives and handles
    /// a stream of block headers sent by the peer in response to a
    /// `BlockEvent::Missing` solicitation.
    type PushHeadersSink: Sink<SinkItem = Self::Header, SinkError = Error> + Send + 'static;

    /// The type of asyncrhonous futures returned by the
    /// `get_push_headers_sink` method.
    ///
    /// The future resolves to a sink that will be used by the application
    /// to handle the stream of incoming headers.
    type GetPushHeadersSinkFuture: Future<Item = Self::PushHeadersSink, Error = Error>
        + Send
        + 'static;

    /// The application-provided sink object that receives blocks
    /// uploaded in response to a `BlockEvent::Solicit` solicitation.
    type UploadBlocksSink: Sink<SinkItem = Self::Block, SinkError = Error> + Send + 'static;

    /// The type of asyncrhonous futures returned by the
    /// `get_upload_blocks_sink` method.
    ///
    /// The future resolves to a sink that will be used by the application
    /// to handle the incoming blocks.
    type GetUploadBlocksSinkFuture: Future<Item = Self::UploadBlocksSink, Error = Error>
        + Send
        + 'static;

    /// The type of asynchronous stream that lets the client receive
    /// new block announcements and solicitation requests from the service.
    type BlockSubscription: Stream<Item = BlockEvent<Self::Block>, Error = Error> + Send + 'static;

    /// The type of asynchronous futures returned by method `block_subscription`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type BlockSubscriptionFuture: Future<Item = Self::BlockSubscription, Error = Error>
        + Send
        + 'static;

    /// Returns the ID of the genesis block of the chain served by this node.
    fn block0(&mut self) -> Self::BlockId;

    /// Request the current blockchain tip.
    /// The returned future resolves to the tip of the blockchain
    /// accepted by this node.
    fn tip(&mut self) -> Self::TipFuture;

    /// Request to load list of blocks.
    fn get_blocks(&mut self, ids: &[Self::BlockId]) -> Self::GetBlocksFuture;

    /// Request to load list of blocks.
    fn get_headers(&mut self, ids: &[Self::BlockId]) -> Self::GetHeadersFuture;

    /// Get blocks, walking forward in a range between either of the given
    /// starting points, and the ending point.
    fn pull_blocks(&mut self, from: &[Self::BlockId], to: &Self::BlockId)
        -> Self::PullBlocksFuture;

    /// Stream blocks from either of the given starting points
    /// to the server's tip.
    fn pull_blocks_to_tip(&mut self, from: &[Self::BlockId]) -> Self::PullBlocksToTipFuture;

    /// Get block headers, walking the chain forward in a range between the
    /// latest among the given starting points, and the given ending point.
    /// If none of the starting points are found in the chain, or if the
    /// ending point is not found, the future will fail with a `NotFound`
    /// error.
    fn pull_headers(
        &mut self,
        from: &[Self::BlockId],
        to: &Self::BlockId,
    ) -> Self::PullHeadersFuture;

    /// Stream block headers from either of the given starting points
    /// to the server's tip.
    fn pull_headers_to_tip(&mut self, from: &[Self::BlockId]) -> Self::PullHeadersFuture;

    /// Called by the protocol implementation to get a sink
    /// that receives and handles a stream of block headers sent by the peer
    /// in response to a `BlockEvent::Missing` solicitation.
    fn get_push_headers_sink(&mut self) -> Self::GetPushHeadersSinkFuture;

    /// Called by the protocol implementation to get a sink
    /// that receives blocks uploaded in response to a `BlockEvent::Solicit`
    /// solicitation.
    fn get_upload_blocks_sink(&mut self) -> Self::GetUploadBlocksSinkFuture;

    /// Establishes a bidirectional subscription for announcing blocks.
    ///
    /// The network protocol implementation passes an asynchronous stream
    /// that will provide the inbound announcements, and an optional
    /// node identifier of the sender that can be correlated with the
    /// gossip information to reuse the connection.
    ///
    /// Returns a future resolving to an asynchronous stream
    /// that will be used by this node to send block announcements
    /// and solicitations.
    fn block_subscription<In>(
        &mut self,
        inbound: In,
        subscriber: Option<Self::NodeId>,
    ) -> Self::BlockSubscriptionFuture
    where
        In: Stream<Item = Self::Header, Error = Error> + Send + 'static;
}
