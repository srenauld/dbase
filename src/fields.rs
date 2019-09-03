use chrono::{Utc, Date, DateTime, TimeZone};
use super::header::{Header, Database, Version};
use std::io;
use std::str::FromStr;
use std::path::PathBuf;
use byteorder::{ReadBytesExt, LittleEndian};
use std::collections::HashMap;
use std::fmt::Debug;
pub trait FieldType:Debug {
    fn parse(&self, database: &mut Database, data: Vec<u8>) -> Result<FieldValue, io::Error>;
}

#[derive(Debug, PartialEq)]
pub enum FieldValue {
    Text(String),
    Numeric(f64),
    Integer(i32), // There's a special type for this
    Boolean(Option<bool>),
    Date(Date<Utc>),
    DateTime(DateTime<Utc>),
    Unknown(Vec<u8>)
}

#[derive(Clone, Debug)]
pub struct FieldTypeC;
impl FieldType for FieldTypeC {
    fn parse(&self, database: &mut Database, data: Vec<u8>) -> Result<FieldValue, io::Error> {
        String::from_utf8(data.clone())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("The field content {:?} cannot be casted to a string", data)))
            .map(|r| FieldValue::Text(r.trim().to_string()))

    }
}

#[derive(Clone, Debug)]
pub struct FieldTypeD;

impl FieldType for FieldTypeD {
    fn parse(&self, database: &mut Database, data: Vec<u8>) -> Result<FieldValue, io::Error> {
        let mut field_content =  String::from_utf8(data.clone())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("The field content {:?} cannot be casted to a string", data)))
            .map(|r| r.trim().to_string())?;
        match field_content.len() {
            8 => {
                let day_str:String = field_content.split_off(6);
                let month_str:String = field_content.split_off(4);
                let day:u32 = FromStr::from_str(&day_str)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, "Month is invalid"))?;
                let month:u32 = FromStr::from_str(&month_str)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, "Month is invalid"))?;
                let year:i32 = FromStr::from_str(&field_content)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, "Year is invalid"))?;
                Ok(FieldValue::Date(Utc.ymd(year, month, day)))
            },
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, format!("The field value {} is not a valid date", field_content)))
        }
    }
}

#[derive(Clone, Debug)]
pub struct FieldTypeOldNumeric;

impl FieldType for FieldTypeOldNumeric {
    fn parse(&self, database: &mut Database, data: Vec<u8>) -> Result<FieldValue, io::Error> {
        String::from_utf8(data.clone())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("The field content {:?} cannot be casted to a string", data)))
            .and_then(|mut data| {
                data.trim_start();
                FromStr::from_str(data.trim_start())
                .map(|r| FieldValue::Numeric(r))
                .map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("The field content {:?} cannot be casted to a float", data))
                })
            })
    }
}

#[derive(Clone, Debug)]
pub struct FieldTypeL;

impl FieldType for FieldTypeL {
    fn parse(&self, database: &mut Database, data: Vec<u8>) -> Result<FieldValue, io::Error> {
        match data.first() {
            Some(r) if *r == 89 || *r == 121 => Ok(FieldValue::Boolean(Some(true))),
            Some(r) if *r == 78 || *r == 110 => Ok(FieldValue::Boolean(Some(false))),
            Some(r) => Ok(FieldValue::Boolean(None)),
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid data for a boolean"))
        }
    }
}

fn vec_u8_to_u32(in_val: Vec<u8>) -> Result<u32, io::Error> {
    let mut reader = io::Cursor::new(in_val);
    reader.read_u32::<LittleEndian>()
}
fn to_julian_date(input: u32) -> Result<Date<Utc>, io::Error> {
    let converted:f64 = input.into();
    let s1:f64 = converted + 68569.0;
    let n:f64 = (4.0 * s1 / 146097.0).floor();
    let s2:f64 = s1 - (((146097.0 * n) + 3.0) / 4.0).floor();
    let i:f64 = (4000.0 * (s2 + 1.0) / 1461001.0).floor();
    let s3:f64 = s2 - (1461.0 * i / 4.0).floor() + 31.0;
    let q = (80.0 * s3 / 2447.0).floor();
    let d = s3 - (2447.0 * q / 80.0).floor();
    let s4 = (q / 11.0).floor();
    let m = q + 2.0 - (12.0 * s4);
    let j = (100.0 * (n - 49.0)) + i + s4;
    Ok(Utc.ymd(j as i32, m as u32, d as u32))
}
#[derive(Clone, Debug)]
pub struct FieldTypeT;
impl FieldType for FieldTypeT {
    fn parse(&self, database: &mut Database, data: Vec<u8>) -> Result<FieldValue, io::Error> {
        let mut dword_iter = data.chunks(4);
        let date_word_vec = dword_iter.next()
            .ok_or(io::Error::new(io::ErrorKind::InvalidData, "Date word not found"))?;
        let time_word_vec = dword_iter.next()
            .ok_or(io::Error::new(io::ErrorKind::InvalidData, "Time word not found"))?;
        let date_word = vec_u8_to_u32(date_word_vec.to_vec())?;
        let time_word = vec_u8_to_u32(time_word_vec.to_vec())?;
        let date = to_julian_date(date_word)?;

        let mut time_word_f64:f64 = time_word.into();
        let hours = (time_word_f64 / 3600000.0).floor();
        time_word_f64 = time_word_f64 - hours * 3600000.0;
        let minutes = (time_word_f64 / 60000.0).floor();
        time_word_f64 = time_word_f64 - minutes * 60000.0;
        let seconds = time_word_f64 / 1000.0;
        Ok(FieldValue::DateTime(date.and_hms(hours as u32, minutes as u32, seconds as u32)))
    }
}

#[derive(Clone, Debug)]
pub struct FieldTypeI;
impl FieldType for FieldTypeI {
    fn parse(&self, database: &mut Database, data: Vec<u8>) -> Result<FieldValue, io::Error> {
        let mut reader = io::Cursor::new(data);
        let integer = reader.read_i32::<LittleEndian>()?;
        Ok(FieldValue::Integer(integer))

    }
}

#[derive(Clone, Debug)]
pub struct FieldTypeM;
impl FieldType for FieldTypeM {
    fn parse(&self, database: &mut Database, data: Vec<u8>) -> Result<FieldValue, io::Error> {
        println!("{:?}", data);
        Ok(FieldValue::Unknown(data))
    }
}

#[test]
fn date_works() {
    let data = vec![0x32, 0x30, 0x31, 0x39, 0x30, 0x39, 0x30, 0x31];
    let mut db = Database::new_at("C:/test.txt");
    let mut o = FieldTypeD;
    assert_eq!(o.parse(&mut db, data).unwrap(), FieldValue::Date(Utc.ymd(2019, 09, 01)));
}

#[test]
fn datetime_works() {
    
    let date = to_julian_date(2458730).unwrap();
    assert_eq!(date, Utc.ymd(2019,09,03));

    let data = vec![0xB8, 0x83, 0x25, 0x00, 0x80, 0xEE, 0x36, 0x00];

    let mut db = Database::new_at("C:/test.txt");
    let mut o = FieldTypeT {};
    assert_eq!(o.parse(&mut db, data).unwrap(), FieldValue::DateTime(Utc.ymd(2019, 03, 09).and_hms(01, 0, 0))); 

}