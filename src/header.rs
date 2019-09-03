use chrono::{Date, Utc, TimeZone};
use std::fs::File;
use super::fields::{FieldType, FieldValue};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::io;
use std::iter::{IntoIterator, Iterator};
use std::io::Read;
use byteorder::{ReadBytesExt, LittleEndian};
use std::str::FromStr;
use std::sync::Arc;
use super::fields;

#[derive(Debug)]
pub enum Version {
    FoxBase,
    dBASE3(bool),
    VisualFoxPro(bool, bool),
    dBASE4Table(bool),
    dBASE4System(bool),
    FoxPro2(bool),
    Unknown
}

impl Version {
    pub fn from_byte(byte: &u8) -> Version {
        match byte {
            0x02 => Self::FoxBase,
            0x03 => Self::dBASE3(false),
            0x30 => Self::VisualFoxPro(false, false),
            0x31 => Self::VisualFoxPro(true, false),
            0x32 => Self::VisualFoxPro(false, true),
            0x33 => Self::VisualFoxPro(true, true),
            0x43 => Self::dBASE4Table(false),
            0x63 => Self::dBASE4System(false),
            0x83 => Self::dBASE3(true),
            0x8b => Self::dBASE4System(true),
            0xcb => Self::dBASE4Table(true),
            0xfb => Self::FoxPro2(false),
            0xf5 => Self::FoxPro2(true),
            0x83 => Self::dBASE3(true),
            _ => Self::Unknown
        }
    }
}

#[derive(Debug, Clone)]
pub struct FieldDescriptor {
    pub name: String,
    pub field_type: Arc<Box<FieldType>>,
    data_address: u32,
    length: u8,
    decimal_count: u8
}

#[derive(Debug)]
pub struct Header {
    pub version: Version,
    pub last_update: Date<Utc>,
    pub record_count: u32,
    header_size: u16,
    record_size: u16,
    fields: Vec<FieldDescriptor>
}

pub struct Database {
    path: PathBuf,
    files: HashMap<String, File>,
    pub header: Header
}

fn parse_date(data: Vec<u8>) -> Result<Date<Utc>, io::Error> {
    match data.len() {
        3 => {
            let year:i32 = (data[0] as i32) + 1900;
            let month = data[1];
            let day = data[2];
            Ok(Utc.ymd(year, month.into(), day.into()))
        },
        _ => Err(io::Error::new(io::ErrorKind::InvalidData, format!("The field value {:?} is not a valid date", data)))
    }
}

#[derive(Debug)]
pub struct Record {
    fields: HashMap<String, FieldValue>
}

pub struct DatabaseRecordIterator {
    database: Database,
    record_size: usize,
    fields: Arc<Vec<FieldDescriptor>>
}

impl DatabaseRecordIterator {
    fn parse_row(&mut self, mut bytes: Vec<u8>) -> Result<Record, io::Error> {
        let fields_clone = Arc::clone(&self.fields);
        let fields:Result<Vec<(String, FieldValue)>, io::Error> = fields_clone.iter().fold(Ok(vec![]), |fields, field| {
            fields.and_then(|mut fields| {
                let record_bytes:Vec<u8> = bytes.drain(0..(field.length as usize)).collect();
                field.field_type.parse(&mut self.database, record_bytes).map(|r| {
                    fields.push((field.name.clone(), r));
                    fields
                })
            })
        });
        fields.map(|fields| {
            Record {
                fields: fields.into_iter().collect()
            }
        })
    }
}

impl Iterator for DatabaseRecordIterator {
    type Item = Record;
    fn next(&mut self) -> Option<Self::Item> {
        // Read the next record
        self.database.files.get("dbf")
            .ok_or(io::Error::new(io::ErrorKind::NotFound, "DBF file wasn't open"))
            .and_then(|mut file| {
                let mut buf = vec![];
                buf.resize(self.record_size, 0);
                file.read_exact(&mut buf)?;
                Ok(buf)
            }).and_then(|buf| {
                self.parse_row(buf)
            }).ok()
    }
}
impl IntoIterator for Database {
    type Item = Record;
    type IntoIter = DatabaseRecordIterator;

    fn into_iter(self) -> Self::IntoIter {
        let fields = self.header.fields.clone();
        let record_size:usize = self.header.fields.iter().fold(0 as usize, |current, field| current + (field.length as usize)) as usize;
        DatabaseRecordIterator {
            database: self,
            record_size: record_size,
            fields: Arc::new(fields)
        }
    }
}
impl Database {
    fn parse_fields(buffer: Vec<u8>) -> Result<Vec<FieldDescriptor>, io::Error> {
        let mut iter = buffer.chunks(32);
        let mut fields = vec![];
        let mut done = false;
        let parse_field = |data:Vec<u8>| -> Result<FieldDescriptor, io::Error> {
            let field_name = String::from_utf8(data[0..11].to_vec())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, "Invalid string"))
                .map(|mut e| {
                    e.trim();
                    e.replace("\0", "")
                })?;
            let field_type_res:Result<Box<fields::FieldType>, io::Error> = match data[11] {
                67 => Ok(Box::new(fields::FieldTypeC)),
                68 => Ok(Box::new(fields::FieldTypeD)),
                70 | 78 => Ok(Box::new(fields::FieldTypeOldNumeric)),
                76 => Ok(Box::new(fields::FieldTypeL)),
                84 => Ok(Box::new(fields::FieldTypeT)),
                73 => Ok(Box::new(fields::FieldTypeI)),
                77 => Ok(Box::new(fields::FieldTypeM)),
                d => Err(io::Error::new(io::ErrorKind::InvalidData, format!("Unknown field type {}", d)))
            };
            let field_type = field_type_res?;
            let field_address = {
                let bytes = data[12..16].to_vec();
                let mut reader = io::Cursor::new(bytes);
                reader.read_u32::<LittleEndian>()?
            };
            let field_length = data[16];
            let field_decimal_count = data[17];
            // We don't really care about the rest
            Ok(FieldDescriptor {
                name: field_name,
                field_type: Arc::new(field_type),
                data_address: field_address,
                length: field_length,
                decimal_count: field_decimal_count
            })
        };
        while !done {
            let next_chunk = iter.next();
            match next_chunk {
                None => done = true,
                Some(r) => match r.first() {
                    Some(r) if *r == 0x0d => {
                        done = true;
                    },
                    _ => {
                        fields.push(parse_field(r.to_vec())?);
                    }
                }
            }
        }
        Ok(fields)
    }
    pub fn parse(path:&str) -> Result<Database, io::Error> {
        let path_buf = PathBuf::from(path);
        let mut file = File::open(path_buf)?;
        let mut byte_header = [0; 12];
        file.read_exact(&mut byte_header)?;
        let version_byte = byte_header.first().ok_or(io::Error::new(io::ErrorKind::NotFound, "No version descriptor"))?;
        let version = Version::from_byte(&version_byte);
        // This is where things get hilarious
        let date_modified = parse_date(byte_header[1..4].to_vec())?;
        let num_records = {
            let bytes = byte_header[4..8].to_vec();
            let mut reader = io::Cursor::new(bytes);
            reader.read_u32::<LittleEndian>()?
        };
        let header_size = {
            let bytes = byte_header[8..10].to_vec();
            let mut reader = io::Cursor::new(bytes);
            reader.read_u16::<LittleEndian>()?
        };
        let record_size = {
            let bytes = byte_header[10..12].to_vec();
            let mut reader = io::Cursor::new(bytes);
            reader.read_u16::<LittleEndian>()?
        };
        {
            let mut wasted_buffer = [0;20];
            let wasted_bytes = file.read_exact(&mut wasted_buffer)?;
        };
        let size:usize = (header_size - 32 + 1).into();
        let mut field_buffer = vec![];
        field_buffer.resize(size, 0);
        file.read_exact(&mut field_buffer)?;

        let fields:Vec<FieldDescriptor> = Self::parse_fields(field_buffer)?;
        Ok(Database {
            path: path.into(),
            files: vec![
                ("dbf".to_string(), file)
            ].into_iter().collect(),
            header: Header {
                version: version,
                last_update: date_modified,
                record_count: num_records,
                header_size: header_size,
                record_size: record_size,
                fields: fields
            }
        })
    }

    pub fn new_at(s: &str) -> Self {
        Database {
            path: PathBuf::from(s),
            files: HashMap::new(),
            header: Header {
                version: Version::Unknown,
                last_update: Utc::now().date(),
                record_count: 0,
                header_size: 0,
                record_size: 0,
                fields: vec![]
            }
        }
    }
}