// #![feature(slicing_syntax)]
extern crate chrono;
extern crate byteorder;

pub mod header;
pub mod fields;

use std::io;
pub use fields::FieldValue;

pub fn open(path: &str) -> Result<header::Database, io::Error> {
    let file = std::fs::File::open(path)?;
    header::Database::parse(path, file)
}
