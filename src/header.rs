use chrono::{Date, Utc, TimeZone};
use std::fs::File;
use super::fields::{FieldType, FieldValue};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::io;
use std::iter::{IntoIterator, Iterator};
use std::io::{Seek, Read};
use byteorder::{ReadBytesExt, BigEndian, LittleEndian};
use std::str::FromStr;
use std::sync::Arc;
use std::fmt::Debug;
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
            0x02 => Version::FoxBase,
            0x03 => Version::dBASE3(false),
            0x30 => Version::VisualFoxPro(false, false),
            0x31 => Version::VisualFoxPro(true, false),
            0x32 => Version::VisualFoxPro(false, true),
            0x33 => Version::VisualFoxPro(true, true),
            0x43 => Version::dBASE4Table(false),
            0x63 => Version::dBASE4System(false),
            0x83 => Version::dBASE3(true),
            0x8b => Version::dBASE4System(true),
            0xcb => Version::dBASE4Table(true),
            0xfb => Version::FoxPro2(false),
            0xf5 => Version::FoxPro2(true),
            _ => Version::Unknown
        }
    }
}

#[derive(Debug, Clone)]
pub struct FieldDescriptor {
    pub name: String,
    pub field_type: Arc<Box<dyn FieldType>>,
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
    descriptor: Option<Box<dyn Read>>,
    pub memo: Option<Box<dyn MemoContainer>>,
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

trait MemoContainer:Debug {
    fn memo(&mut self, id: Vec<u8>) -> Result<Vec<u8>, io::Error>;
}

#[derive(Debug)]
pub struct FoxProMemoContainer {
    descriptor: File,
    fragment_size: u32,
    block_size: u32
}
impl FoxProMemoContainer {
    pub fn open<T:AsRef<Path>>(path: T) -> Result<Self, io::Error> {
        let mut file = File::open(path)?;
        let mut buf = vec![];
        buf.resize(8, 0);
        file.read_exact(&mut buf)?;
        let next_available = {
            let bytes = buf[0..4].to_vec();
            let mut reader = io::Cursor::new(bytes);
            reader.read_u32::<LittleEndian>()?
        };
        let block_size = {
            let bytes = buf[4..6].to_vec();
            let mut reader = io::Cursor::new(bytes);
            match reader.read_u16::<LittleEndian>()? {
                0 => 512,
                v => v
            }
        };
        let fragment_size = {
            let bytes = buf[7];
            bytes

        };
        Ok(FoxProMemoContainer {
            descriptor: file,
            fragment_size: fragment_size as u32,
            block_size: block_size as u32
        })
    }
}
impl MemoContainer for FoxProMemoContainer {
    fn memo(&mut self, data:Vec<u8>) -> Result<Vec<u8>, io::Error> {
        let id:u32 = {
            let mut reader = io::Cursor::new(data);
            reader.read_u32::<LittleEndian>()?
        };
        self.descriptor.seek(io::SeekFrom::Start((self.fragment_size as u64)* (id as u64)))?;
        let data_type = {
            let mut buf_header = vec![];
            buf_header.resize(4, 0);
            self.descriptor.read_exact(&mut buf_header)?;
            buf_header[2]
        };
        // Seek another 4 bytes to get the length of the memo
        let memo_len = {
            let mut buf_length = vec![];
            buf_length.resize(4, 0);
            self.descriptor.read_exact(&mut buf_length)?;
            let mut reader = io::Cursor::new(buf_length);
            reader.read_u32::<BigEndian>()?
        };
        // Read the memo
        let mut memo_buf = vec![];
        memo_buf.resize(memo_len as usize, 0);
        self.descriptor.read_exact(&mut memo_buf)?;
        Ok(memo_buf)
    }
}
#[derive(Debug)]
pub struct DBaseMemoContainer {
    descriptor: File,
    block_size: usize,
    next_available: usize
}
impl DBaseMemoContainer {
    pub fn open<T:AsRef<Path>>(path: T) -> Result<Self, io::Error> {
        let mut file = File::open(path)?;
        let mut buf = vec![];
        buf.resize(8, 0);
        file.read_exact(&mut buf)?;
        let next_available = {
            let bytes = buf[0..4].to_vec();
            let mut reader = io::Cursor::new(bytes);
            reader.read_u32::<LittleEndian>()?
        };
        let block_size = {
            let bytes = buf[4..6].to_vec();
            let mut reader = io::Cursor::new(bytes);
            match reader.read_u16::<LittleEndian>()? {
                0 => 512,
                v => v
            }
        };
        Ok(DBaseMemoContainer {
            descriptor: file,
            block_size: block_size as usize,
            next_available: next_available as usize
        })
    }
}
impl MemoContainer for DBaseMemoContainer {
    fn memo(&mut self, data: Vec<u8>) -> Result<Vec<u8>, io::Error> {
        let id:u32 = {
            String::from_utf8(data.clone())
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, format!("The field content {:?} cannot be casted to a string", data)))
                .and_then(|data_str| {
                    FromStr::from_str(data_str.trim_start())
                    .map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidData, format!("The field content {:?} cannot be casted to a float", data))
                    })
                })?
        };
        self.descriptor.seek(io::SeekFrom::Start((self.block_size as u64) * (id as u64)))?;
        let mut memo_bytes = vec![];
        let mut done = false;
        while !done {
            let mut bytes = vec![];
            bytes.resize(self.block_size, 0);
            let bytes_read = self.descriptor.read(&mut bytes)?;
            done = bytes_read < self.block_size || bytes.contains(&0x1a);
            memo_bytes.append(&mut bytes);
        }
        let mut new_bytes:Vec<Vec<u8>> = memo_bytes.rsplitn(2, |n| *n == 0x1a).map(|r| r.to_vec()).collect();
        if new_bytes.len() > 1 {
            new_bytes.reverse();
            new_bytes.pop();
        }
        let mut output = vec![];
        new_bytes.into_iter().for_each(|mut e| output.append(&mut e));
        match output.last() {
            Some(r) if *r == 0x1a => { output.pop(); },
            _ => ()
        }
        Ok(output)
    }
}

#[derive(Debug)]
pub struct Record {
    pub fields: HashMap<String, FieldValue>
}
impl Record {
    pub fn get(&self, field: &str) -> Option<&FieldValue> {
        self.fields.get(&field.to_string())
    }
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
                }).map_err(|e| {
                    e
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
        self.database.read_bytes(self.record_size)
        .and_then(|buf| {
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
    fn read_bytes(&mut self, count: usize) -> Result<Vec<u8>, io::Error> {
        self.descriptor.as_mut().ok_or(io::Error::new(io::ErrorKind::NotFound, "No descriptor"))
        .and_then(|file| {
            let mut buf = vec![];
            buf.resize(count+1, 0);
            file.read_exact(&mut buf)?;
            Ok(buf)
        })
    }
    fn parse_fields(buffer: Vec<u8>) -> Result<Vec<FieldDescriptor>, io::Error> {
        let mut iter = buffer.chunks(32);
        let mut fields = vec![];
        let mut done = false;
        let parse_field = |data:Vec<u8>| -> Result<FieldDescriptor, io::Error> {
            let field_name = String::from_utf8(data[0..11].to_vec())
                .map_err(|_e| io::Error::new(io::ErrorKind::InvalidInput, "Invalid string"))
                .map(|e| {
                    e.trim().replace("\0", "")
                })?;
            let field_type_res:Result<Box<dyn fields::FieldType>, io::Error> = match data[11] {
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
    pub fn parse(path: &str, mut file: impl Read + 'static) -> Result<Database, io::Error> {
        let mut byte_header = [0; 12];
        let file_path = PathBuf::from(path);
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
            file.read_exact(&mut wasted_buffer)?;
        };
        let size:usize = (header_size - 32 + 1).into();
        let mut field_buffer = vec![];
        field_buffer.resize(size, 0);
        file.read_exact(&mut field_buffer)?;

        let fields:Vec<FieldDescriptor> = Self::parse_fields(field_buffer)?;

        // Do we have a memo file?
        let stem = file_path.file_stem().and_then(|r| r.to_str()).map(|r| r.to_string()).unwrap_or("".to_string());
        let mut dir_path:Vec<_> = file_path.components().map(|r| r.as_os_str()).collect();
        dir_path.pop();
        let mut dir = PathBuf::new();
        dir_path.into_iter().for_each(|component| dir.push(component));

        let memo_file:Option<Box<MemoContainer>> = {
            let mut dbt_pathbuf = dir.clone();
            dbt_pathbuf.push(format!("{}.dbt", stem));
            match dbt_pathbuf.is_file() {
                true => {
                    Some(Box::new(DBaseMemoContainer::open(dbt_pathbuf)?))
                },
                false => {
                    let mut fpt_pathbuf = dir.clone();
                    fpt_pathbuf.push(format!("{}.fpt", stem));
                    match fpt_pathbuf.is_file() {
                        true => {
                            Some(Box::new(FoxProMemoContainer::open(fpt_pathbuf)?))
                        },
                        false => None
                    }
                }
            }
        };

        Ok(Database {
            path: path.into(),
            memo: memo_file,
            descriptor: Some(Box::new(file)),
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

    pub fn get_memo(&mut self, data: Vec<u8>) -> Option<Vec<u8>> {
        self.memo.as_mut().and_then(|container| {
            container.memo(data).ok()
        })
    }
    pub fn new_at(s: &str) -> Self {
        Database {
            path: PathBuf::from(s),
            memo: None,
            descriptor: None,
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