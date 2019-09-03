// #![feature(slicing_syntax)]
extern crate chrono;
extern crate byteorder;

pub mod header;
pub mod fields;
use std::path::Path;
use std::io;

pub fn open(file: &str) -> Result<header::Database, io::Error> {
    header::Database::parse(file)
}
