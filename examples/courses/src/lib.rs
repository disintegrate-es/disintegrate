pub mod application;
pub mod domain;
pub mod grpc;
pub mod postgres;
pub mod read_model;
pub mod serde;

pub mod proto {
    tonic::include_proto!("event");
    tonic::include_proto!("api");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("courses_descriptor");
}
