//! Reads the unencrypted framing of MLS messages (RFC 9420 §6), which is
//! enough to keep each group's commits in one order without its keys.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    Application,
    Proposal,
    Commit,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Framing {
    pub group_id: Vec<u8>,
    pub epoch: u64,
    pub content_type: ContentType,
}

const MLS10: u16 = 1;
const PUBLIC_MESSAGE: u16 = 1;
const PRIVATE_MESSAGE: u16 = 2;

/// The group, epoch and content type of a serialized `MLSMessage`, `None`
/// when it is malformed or not a public or private message (e.g. a welcome).
pub fn framing(bytes: &[u8]) -> Option<Framing> {
    let mut r = Reader { bytes, offset: 0 };
    if r.u16()? != MLS10 {
        return None;
    }
    let wire_format = r.u16()?;
    if wire_format != PUBLIC_MESSAGE && wire_format != PRIVATE_MESSAGE {
        return None;
    }
    let group_id = r.vector()?.to_vec();
    let epoch = r.u64()?;
    if wire_format == PUBLIC_MESSAGE {
        // FramedContent puts the sender and authenticated data before the
        // content type, PrivateMessage encrypts the sender
        match r.u8()? {
            // member, external: a leaf or sender index
            1 | 2 => r.skip(4)?,
            // new member proposal, new member commit
            3 | 4 => {}
            _ => return None,
        }
        r.vector()?;
    }
    let content_type = match r.u8()? {
        1 => ContentType::Application,
        2 => ContentType::Proposal,
        3 => ContentType::Commit,
        _ => return None,
    };
    Some(Framing {
        group_id,
        epoch,
        content_type,
    })
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.offset.checked_add(n)?;
        let slice = self.bytes.get(self.offset..end)?;
        self.offset = end;
        Some(slice)
    }

    fn skip(&mut self, n: usize) -> Option<()> {
        self.take(n).map(|_| ())
    }

    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_be_bytes(self.take(2)?.try_into().ok()?))
    }

    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_be_bytes(self.take(8)?.try_into().ok()?))
    }

    /// A vector with an RFC 9420 §2.1.2 variable-length size prefix.
    fn vector(&mut self) -> Option<&'a [u8]> {
        let first = self.u8()?;
        let size = match first >> 6 {
            0 => 1,
            1 => 2,
            2 => 4,
            _ => return None,
        };
        let mut length = (first & 0x3f) as usize;
        for &b in self.take(size - 1)? {
            length = (length << 8) | b as usize;
        }
        self.take(length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn private_message(content_type: u8) -> Vec<u8> {
        let mut bytes = vec![0, 1, 0, 2];
        bytes.extend([3, 0xaa, 0xbb, 0xcc]); // group id
        bytes.extend(7u64.to_be_bytes());
        bytes.push(content_type);
        bytes.extend([0, 1, 0x11, 1, 0x22]); // aad, sender data, ciphertext
        bytes
    }

    #[test]
    fn reads_a_private_message() {
        assert_eq!(
            framing(&private_message(3)),
            Some(Framing {
                group_id: vec![0xaa, 0xbb, 0xcc],
                epoch: 7,
                content_type: ContentType::Commit,
            })
        );
        assert_eq!(
            framing(&private_message(1)).unwrap().content_type,
            ContentType::Application
        );
    }

    #[test]
    fn reads_a_public_message_past_the_sender() {
        let mut bytes = vec![0, 1, 0, 1];
        // a two byte size prefix
        bytes.extend([0x40, 2, 0x01, 0x02]);
        bytes.extend(42u64.to_be_bytes());
        bytes.extend([1, 0, 0, 0, 5]); // member sender at leaf 5
        bytes.extend([2, 0x33, 0x44]); // authenticated data
        bytes.push(3);
        let f = framing(&bytes).unwrap();
        assert_eq!(f.group_id, vec![1, 2]);
        assert_eq!(f.epoch, 42);
        assert_eq!(f.content_type, ContentType::Commit);

        let mut external = vec![0, 1, 0, 1, 1, 9];
        external.extend(3u64.to_be_bytes());
        external.extend([4, 0, 3]); // new member commit, empty aad
        assert_eq!(
            framing(&external).unwrap().content_type,
            ContentType::Commit
        );
    }

    #[test]
    fn rejects_other_and_truncated_messages() {
        assert_eq!(framing(&[0, 1, 0, 3, 0]), None); // welcome
        assert_eq!(framing(&[0, 2, 0, 2]), None); // unknown version
        let bytes = private_message(3);
        for end in 0..13 {
            assert_eq!(framing(&bytes[..end]), None, "{end} bytes");
        }
        assert_eq!(framing(&private_message(9)), None);
    }
}
