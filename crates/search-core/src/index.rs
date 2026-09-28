use std::io::{self, Read, Write};

pub const INDEX_MAGIC: [u8; 8] = *b"STIDX\0\0\0";
pub const INDEX_VERSION: u16 = 1;
pub const FILE_RECORD_V1_SIZE: u16 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexHeader {
    pub record_count: u64,
    pub string_pool_bytes: u64,
}

impl IndexHeader {
    pub const ENCODED_SIZE: usize = 32;

    pub fn write_to<W: Write>(&self, mut out: W) -> io::Result<()> {
        out.write_all(&INDEX_MAGIC)?;
        out.write_all(&INDEX_VERSION.to_le_bytes())?;
        out.write_all(&FILE_RECORD_V1_SIZE.to_le_bytes())?;
        out.write_all(&self.record_count.to_le_bytes())?;
        out.write_all(&self.string_pool_bytes.to_le_bytes())?;
        out.write_all(&[0_u8; 4])?;
        Ok(())
    }

    pub fn read_from<R: Read>(mut input: R) -> io::Result<Self> {
        let mut magic = [0_u8; 8];
        input.read_exact(&mut magic)?;
        if magic != INDEX_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid index magic",
            ));
        }

        let version = read_u16(&mut input)?;
        if version != INDEX_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported index version",
            ));
        }

        let record_size = read_u16(&mut input)?;
        if record_size != FILE_RECORD_V1_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected record size",
            ));
        }

        let record_count = read_u64(&mut input)?;
        let string_pool_bytes = read_u64(&mut input)?;

        let mut reserved = [0_u8; 4];
        input.read_exact(&mut reserved)?;

        Ok(Self {
            record_count,
            string_pool_bytes,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileRecordV1 {
    pub file_id: u64,
    pub parent_id: u64,
    pub size_bytes: u64,
    pub name_id: u32,
    pub extension_id: u16,
    pub flags: u16,
}

impl FileRecordV1 {
    pub const ENCODED_SIZE: usize = FILE_RECORD_V1_SIZE as usize;

    pub fn write_to<W: Write>(&self, mut out: W) -> io::Result<()> {
        out.write_all(&self.file_id.to_le_bytes())?;
        out.write_all(&self.parent_id.to_le_bytes())?;
        out.write_all(&self.size_bytes.to_le_bytes())?;
        out.write_all(&self.name_id.to_le_bytes())?;
        out.write_all(&self.extension_id.to_le_bytes())?;
        out.write_all(&self.flags.to_le_bytes())?;
        Ok(())
    }

    pub fn read_from<R: Read>(mut input: R) -> io::Result<Self> {
        Ok(Self {
            file_id: read_u64(&mut input)?,
            parent_id: read_u64(&mut input)?,
            size_bytes: read_u64(&mut input)?,
            name_id: read_u32(&mut input)?,
            extension_id: read_u16(&mut input)?,
            flags: read_u16(&mut input)?,
        })
    }
}

pub mod flags {
    pub const DIRECTORY: u16 = 1 << 0;
    pub const HIDDEN: u16 = 1 << 1;
    pub const SYSTEM: u16 = 1 << 2;
    pub const REPARSE_POINT: u16 = 1 << 3;
}

fn read_u16<R: Read>(input: &mut R) -> io::Result<u16> {
    let mut bytes = [0_u8; 2];
    input.read_exact(&mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32<R: Read>(input: &mut R) -> io::Result<u32> {
    let mut bytes = [0_u8; 4];
    input.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64<R: Read>(input: &mut R) -> io::Result<u64> {
    let mut bytes = [0_u8; 8];
    input.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_roundtrip_is_stable() {
        let header = IndexHeader {
            record_count: 12_345,
            string_pool_bytes: 987_654,
        };
        let mut bytes = Vec::new();
        header.write_to(&mut bytes).unwrap();
        assert_eq!(bytes.len(), IndexHeader::ENCODED_SIZE);
        assert_eq!(IndexHeader::read_from(bytes.as_slice()).unwrap(), header);
    }

    #[test]
    fn file_record_is_exactly_32_bytes_on_disk() {
        let record = FileRecordV1 {
            file_id: 1,
            parent_id: 2,
            size_bytes: 3,
            name_id: 4,
            extension_id: 5,
            flags: flags::DIRECTORY,
        };
        let mut bytes = Vec::new();
        record.write_to(&mut bytes).unwrap();
        assert_eq!(bytes.len(), FileRecordV1::ENCODED_SIZE);
        assert_eq!(FileRecordV1::read_from(bytes.as_slice()).unwrap(), record);
    }
}
