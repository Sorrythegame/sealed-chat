//! Message content: a list of mixed blocks (text / image). Serialized to JSON
//! and encrypted end-to-end before it ever leaves the client.

use serde::{Deserialize, Serialize};

/// Full plaintext message content: an ordered list of mixed blocks.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct MessageContent {
    pub blocks: Vec<Block>,
}

impl MessageContent {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            blocks: vec![Block::Text { text: text.into() }],
        }
    }
}

/// A single piece of a message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    Text {
        text: String,
    },
    Image {
        /// Server-side attachment id of the encrypted image.
        attachment_id: String,
        /// The image's symmetric key, encrypted with the session key (base64).
        key_ciphertext: String,
        /// Nonce used to encrypt `key_ciphertext` (base64).
        key_nonce: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_roundtrip() {
        let content = MessageContent {
            blocks: vec![
                Block::Text {
                    text: "hello".into(),
                },
                Block::Image {
                    attachment_id: "abc".into(),
                    key_ciphertext: "k".into(),
                    key_nonce: "n".into(),
                },
            ],
        };
        let json = serde_json::to_string(&content).unwrap();
        let back: MessageContent = serde_json::from_str(&json).unwrap();
        assert_eq!(content, back);
    }
}
