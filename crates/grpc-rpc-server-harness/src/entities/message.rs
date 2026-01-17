/// Represents a gRPC message (request or response)
#[derive(Debug, Clone)]
pub struct Message {
    pub data: Vec<u8>,
}

impl Message {
    pub fn new(data: impl Into<Vec<u8>>) -> Self {
        Self { data: data.into() }
    }

    pub fn empty() -> Self {
        Self { data: Vec::new() }
    }

    pub fn from_prost<T: prost::Message>(msg: &T) -> Self {
        Self {
            data: msg.encode_to_vec(),
        }
    }

    pub fn decode<T: prost::Message + Default>(&self) -> Result<T, prost::DecodeError> {
        T::decode(self.data.as_slice())
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl From<Vec<u8>> for Message {
    fn from(data: Vec<u8>) -> Self {
        Self::new(data)
    }
}

impl From<Message> for Vec<u8> {
    fn from(msg: Message) -> Self {
        msg.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_new() {
        let msg = Message::new(vec![1, 2, 3]);
        assert_eq!(msg.data, vec![1, 2, 3]);
    }

    #[test]
    fn test_message_empty() {
        let msg = Message::empty();
        assert!(msg.is_empty());
    }

    #[test]
    fn test_message_from_vec() {
        let msg: Message = vec![1, 2, 3].into();
        assert_eq!(msg.data, vec![1, 2, 3]);
    }

    #[test]
    fn test_message_into_vec() {
        let msg = Message::new(vec![1, 2, 3]);
        let data: Vec<u8> = msg.into();
        assert_eq!(data, vec![1, 2, 3]);
    }
}
