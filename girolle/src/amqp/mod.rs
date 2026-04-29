//! AMQP transport layer.
//!
//! Owns every direct interaction with `lapin`: opening connections,
//! declaring channels/queues/exchanges, manipulating Nameko-specific
//! AMQP headers, and publishing reply messages. Higher layers
//! (`service`, `client`) talk to the broker exclusively through this
//! module.

pub(crate) mod channel;
pub(crate) mod connection;
pub(crate) mod headers;
pub(crate) mod publish;
