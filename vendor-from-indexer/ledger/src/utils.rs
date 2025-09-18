use crate::serialize::{Deserializable, Serializable};
use std::io::{self, Read, Write};

pub(crate) struct CapturingReader<R: Read> {
    inner: R,
    pub(crate) data: Vec<u8>,
}

impl<R: Read> Read for CapturingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buf)?;

        self.data.extend(&buf[..read]);
        Ok(read)
    }
}

impl<R: Read> CapturingReader<R> {
    pub(crate) fn new(reader: R) -> Self {
        CapturingReader {
            inner: reader,
            data: Vec::new(),
        }
    }

    pub(crate) fn with_data(reader: R, data: Vec<u8>) -> Self {
        CapturingReader {
            inner: reader,
            data,
        }
    }

    pub(crate) fn into_inner(self) -> (R, Vec<u8>) {
        (self.inner, self.data)
    }
}

pub(crate) fn deserialize_and_capture<T: Deserializable, R: Read>(
    reader: &mut R,
    recursion_depth: u32,
) -> io::Result<(T, Vec<u8>)> {
    let mut reader = CapturingReader::new(reader);
    let res = Deserializable::deserialize(&mut reader, recursion_depth)?;
    let (_, data) = reader.into_inner();
    Ok((res, data))
}

pub(crate) fn serialize_or_data_mem<T: Serializable, W: Write, U: AsRef<[u8]> + ?Sized>(
    var: &T,
    data: Option<&U>,
    writer: &mut W,
) {
    serialize_or_data(var, data, writer).expect("In-memory write should succeed");
}

pub(crate) fn serialize_or_data<T: Serializable, W: Write, U: AsRef<[u8]> + ?Sized>(
    var: &T,
    data: Option<&U>,
    writer: &mut W,
) -> io::Result<()> {
    if let Some(data) = data {
        writer.write_all(data.as_ref())
    } else {
        Serializable::serialize(var, writer)
    }
}
