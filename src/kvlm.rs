// Key-Value List with Message
use indexmap::IndexMap;

/// Represents a key-value list with an optional message body.
///
/// This structure preserves insertion order of keys using `IndexMap`,
/// which is important for serialization correctness.
///
/// Note:
/// - Duplicate keys must appear consecutively for order to be preserved.
/// - The message body (if any) is stored with a `None` key.
pub struct Kvlm {
    /// Maps keys (optional `String`) to one or more values (`Vec<String>`).
    /// The `None` key is reserved for the message body.
    data: IndexMap<Option<String>, Vec<String>>,
}

impl Kvlm {
    /// Creates a new `Kvlm` by parsing raw byte data.
    pub fn new(raw_data: &[u8]) -> Self {
        Self {
            data: Self::parse(raw_data),
        }
    }

    /// Creates an empty `Kvlm`.
    pub fn init() -> Self {
        Self {
            data: IndexMap::new(),
        }
    }

    /// Serializes the key-value data (including the message if present) into a byte vector.
    ///
    /// Keys and values are serialized with continuation lines encoded with a leading space.
    /// The message body (key = `None`) is appended after a blank line.
    pub fn serialize(&self) -> Vec<u8> {
        let mut output = Vec::new();

        for (key, values) in &self.data {
            if let Some(field) = key {
                for value in values {
                    // Encode continuation lines by replacing `\n` with `\n `
                    // and accumulate in a temporary buffer.
                    let mut encoded = Vec::with_capacity(value.len());
                    for (i, part) in value.split('\n').enumerate() {
                        if i > 0 {
                            encoded.extend_from_slice(b"\n ");
                        }
                        encoded.extend_from_slice(part.as_bytes());
                    }

                    output.extend_from_slice(field.as_bytes());
                    output.push(b' ');
                    output.extend_from_slice(&encoded);
                    output.push(b'\n');
                }
            }
        }

        // Append the message body after a blank line, if it exists.
        if let Some(messages) = self.data.get(&None) {
            if let Some(message) = messages.first() {
                output.push(b'\n');
                output.extend_from_slice(message.as_bytes());
            }
        }

        output
    }

    /// Returns a reference to the message body string.
    pub fn get_message(&self) -> &str {
        self.data
            .get(&None)
            .expect("Each Kvlm must specify a message")
            .first()
            .expect("Cannot have an empty message")
            .as_str()
    }

    /// Returns a reference to the list of values associated with a given key, if present.
    pub fn get_key(&self, key: &str) -> Option<&Vec<String>> {
        self.data.get(&Some(key.to_string()))
    }

    /// Inserts a single value for the specified key, replacing any existing values.
    pub fn insert(&mut self, key: &str, value: &str) {
        self.data
            .insert(Some(key.to_string()), vec![value.to_string()]);
    }

    /// Parses raw byte data into an `IndexMap` of key-value pairs with an optional message.
    ///
    /// Expects data to be formatted with lines of the form `key value`, continuation
    /// lines starting with a space, and an optional message separated by a blank line.
    fn parse(raw_data: &[u8]) -> IndexMap<Option<String>, Vec<String>> {
        let mut pos = 0;
        let mut result: IndexMap<Option<String>, Vec<String>> = IndexMap::new();

        while pos < raw_data.len() {
            // Check for blank line separating headers from message body
            if raw_data[pos] == b'\n' {
                let message = String::from_utf8_lossy(&raw_data[pos + 1..]).into_owned();
                result.insert(None, vec![message]);
                break;
            }

            // Find space separating key and value
            let space_idx = raw_data[pos..]
                .iter()
                .position(|&b| b == b' ')
                .map(|i| i + pos)
                .expect("Expected space after key");

            // Extract key as UTF-8 string
            let key = String::from_utf8_lossy(&raw_data[pos..space_idx]).into_owned();

            // Find the end of the value, including continuation lines starting with space
            let mut end = space_idx;
            loop {
                let newline_idx = raw_data[end + 1..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .map(|i| i + end + 1)
                    .expect("Expected newline");

                end = newline_idx;

                // If next line doesn't start with space, break
                if raw_data.get(newline_idx + 1) != Some(&b' ') {
                    break;
                }
            }

            // Extract the raw value slice, including continuation lines
            let value_raw_data = &raw_data[space_idx + 1..=end];
            // Replace continuation indent "\n " with "\n"
            let value = String::from_utf8_lossy(value_raw_data)
                .replace("\n ", "\n")
                .to_string();

            // Insert or append to the key's vector of values
            result
                .entry(Some(key))
                .and_modify(|v| v.push(value.clone()))
                .or_insert_with(|| vec![value]);

            pos = end + 1;
        }

        result
    }
}
