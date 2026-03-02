#![allow(clippy::result_large_err)]
extern crate prost;

pub mod database_driver_v1 {
    include!(concat!(env!("OUT_DIR"), "/database_driver_v1.rs"));
}
