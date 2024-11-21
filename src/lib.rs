pub mod config;
pub mod protocol {
    pub mod decoder;
    pub mod encoder;

    #[cfg(test)]
    pub(crate) mod encoder_test;

    #[cfg(test)]
    pub(crate) mod decoder_test;
}
pub mod server;
pub mod store;
