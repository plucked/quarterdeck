//! `protocol_lifecycle` checks SDK interoperability; `http_security` checks
//! request rejection; `server_shutdown` checks owned lifetime and concurrency.

mod http_security;
mod protocol_lifecycle;
mod server_shutdown;
