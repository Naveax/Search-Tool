use std::fs;
use std::io;
use std::path::Path;

const MAGIC: [u8; 8] = *b"STAI1\0\0\0";
const VERSION: u16 = 1;
const HEADER_SIZE: usize = 64;
const MAX_FEATURES: usize = 96;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum QueryIntent {
    ExactSearch = 0,
    FuzzySearch = 1,
    RelatedSearch = 2,
    ContentSearch = 3,
    CleanupAnalysis = 4,
    WebLookup = 5,
    Help = 6,
    Unknown = 7,
}

impl QueryIntent {
    fn from_index(index: usize) -> Self {
        match index {
            0 => Self::ExactSearch,
            1 => Self::FuzzySearch,
            2 => Self::RelatedSearch,
            3 => Self::ContentSearch,
            4 => Self::CleanupAnalysis,
            5 => Self::WebLookup,
            6 => Self::Help,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntentPrediction {
    pub intent: QueryIntent,
    pub confidence: u8,
    pub margin: i32,
}

#[derive(Debug)]
pub struct TinyIntentModel {
    classes: usize,
    buckets: usize,
    biases: Vec<i32>,
    weights: Vec<i8>,
}

impl TinyIntentModel {
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self> {
        let bytes = fs::read(path)?;
        Self::from_bytes(&bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() < HEADER_SIZE || bytes[..8] != MAGIC {
            return Err(invalid("invalid tiny model header"));
        }
        let version = u16::from_le_bytes([bytes[8], bytes[9]]);
        if version != VERSION {
            return Err(invalid("unsupported tiny model version"));
        }
        let classes = u16::from_le_bytes([bytes[10], bytes[11]]) as usize;
        let buckets = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
        let params = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) as usize;
        if classes == 0 || classes > 32 || buckets == 0 || params != classes.saturating_mul(buckets)
        {
            return Err(invalid("invalid tiny model dimensions"));
        }
        let bias_bytes = classes
            .checked_mul(4)
            .ok_or_else(|| invalid("model overflow"))?;
        if 20 + bias_bytes > HEADER_SIZE || bytes.len() != HEADER_SIZE + params {
            return Err(invalid("tiny model size mismatch"));
        }
        let mut biases = Vec::with_capacity(classes);
        for i in 0..classes {
            let at = 20 + i * 4;
            biases.push(i32::from_le_bytes([
                bytes[at],
                bytes[at + 1],
                bytes[at + 2],
                bytes[at + 3],
            ]));
        }
        let weights = bytes[HEADER_SIZE..].iter().map(|&x| x as i8).collect();
        Ok(Self {
            classes,
            buckets,
            biases,
            weights,
        })
    }

    pub fn parameters(&self) -> usize {
        self.weights.len()
    }
    pub fn resident_bytes(&self) -> usize {
        self.weights.capacity() + self.biases.capacity() * 4
    }

    pub fn classify(&self, query: &str) -> IntentPrediction {
        let features = feature_buckets(query, self.buckets);
        let mut best = (usize::MAX, i32::MIN);
        let mut second = i32::MIN;
        for class in 0..self.classes {
            let base = class * self.buckets;
            let mut score = self.biases[class];
            for &feature in &features {
                score += self.weights[base + feature] as i32;
            }
            if score > best.1 {
                second = best.1;
                best = (class, score);
            } else if score > second {
                second = score;
            }
        }
        let margin = if second == i32::MIN {
            best.1.max(0)
        } else {
            best.1.saturating_sub(second)
        };
        let confidence = (50_i32 + margin.saturating_mul(3)).clamp(0, 99) as u8;
        IntentPrediction {
            intent: QueryIntent::from_index(best.0),
            confidence,
            margin,
        }
    }
}

fn feature_buckets(input: &str, buckets: usize) -> Vec<usize> {
    let normalized: String = input
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() {
                ch.to_lowercase().next().unwrap_or(ch)
            } else {
                ' '
            }
        })
        .collect();
    let words: Vec<&str> = normalized.split_whitespace().collect();
    let mut features = Vec::with_capacity(MAX_FEATURES);
    for word in &words {
        push_unique(&mut features, bucket(&format!("w:{word}"), buckets));
        let chars: Vec<char> = word.chars().collect();
        if chars.len() >= 3 {
            for window in chars.windows(3) {
                let tri: String = window.iter().collect();
                push_unique(&mut features, bucket(&format!("c3:{tri}"), buckets));
                if features.len() >= MAX_FEATURES {
                    return features;
                }
            }
        }
    }
    for pair in words.windows(2) {
        push_unique(
            &mut features,
            bucket(&format!("b:{}_{}", pair[0], pair[1]), buckets),
        );
        if features.len() >= MAX_FEATURES {
            break;
        }
    }
    features
}

fn push_unique(features: &mut Vec<usize>, value: usize) {
    if features.len() < MAX_FEATURES && !features.contains(&value) {
        features.push(value);
    }
}

fn bucket(value: &str, buckets: usize) -> usize {
    (fnv1a(value.as_bytes()) % buckets as u64) as usize
}
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_bad_model() {
        assert!(TinyIntentModel::from_bytes(b"bad").is_err());
    }
    #[test]
    fn feature_hash_is_stable() {
        assert_eq!(fnv1a(b"w:node"), 0x4383bee8b3b277a0);
    }
}
