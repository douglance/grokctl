//! Canonical JSON hashing for idempotency witnesses.

use serde_json::Value;
use sha2::{Digest, Sha256};

/// Hash JSON after recursively sorting object keys.
#[must_use]
pub fn canonical_json_hash(value: &Value) -> String {
    let bytes = serde_json::to_vec(&canonicalize(value)).unwrap_or_default();
    format!("{:x}", Sha256::digest(bytes))
}

fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(canonicalize).collect()),
        Value::Object(values) => {
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort_unstable();
            let entries =
                keys.into_iter().map(|key| (key.clone(), canonicalize(&values[key]))).collect();
            Value::Object(entries)
        }
        scalar => scalar.clone(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn object_key_order_does_not_change_hash() {
        let left = json!({"a": 1, "b": {"c": 2, "d": 3}});
        let right = json!({"b": {"d": 3, "c": 2}, "a": 1});
        let changed = json!({"a": 2, "b": {"c": 2, "d": 3}});

        assert_eq!(canonical_json_hash(&left), canonical_json_hash(&right));
        assert_ne!(canonical_json_hash(&left), canonical_json_hash(&changed));
    }
}
