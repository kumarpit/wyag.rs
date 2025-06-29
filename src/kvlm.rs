// Key-Value List with Message
use indexmap::IndexMap;

pub struct Kvlm {
    // Using an IndexMap instead of a regular HashMap to ensure that insertion order is preserved.
    // This is crucial to ensure we don't end up changing the order of the keys when serializing,
    // ending up with the multiple equivalent objects.
    // WARN: In the case of duplicated keys, they MUST appear one after the other. If duplicated
    // keys appear interleaved with other keys, it is not possible for the IndexMap to maintain
    // insertion order.
    data: IndexMap<Option<String>, Vec<String>>,
}

impl Kvlm {
    pub fn new(raw_data: &[u8]) -> Self {
        Self {
            data: Self::parse(raw_data),
        }
    }

    pub fn deserialize(&self) -> Vec<u8> {
        let mut output = Vec::new();

        for (key, values) in &self.data {
            if let Some(field) = key {
                for value in values {
                    // Replace `\n` with `\n `
                    let mut encoded = Vec::with_capacity(value.len());
                    for (i, part) in value.split(|ch| ch == '\n').enumerate() {
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

        // Append the message (None key), if it exists
        if let Some(messages) = self.data.get(&None) {
            if let Some(message) = messages.first() {
                // Don't need to iterate since there
                // is always just 1 message
                output.push(b'\n');
                output.extend_from_slice(message.as_bytes());
            }
        }

        output
    }

    fn parse(raw: &[u8]) -> IndexMap<Option<String>, Vec<String>> {
        let mut pos = 0;
        let mut result: IndexMap<Option<String>, Vec<String>> = IndexMap::new();

        while pos < raw.len() {
            // Check for the blank line separator (`\n`) -> message body
            if raw[pos] == b'\n' {
                let message = String::from_utf8_lossy(&raw[pos + 1..]).into_owned();
                result.insert(None, vec![message]);
                break;
            }

            // Parse key
            let space_idx = raw[pos..]
                .iter()
                .position(|&b| b == b' ')
                .map(|i| i + pos)
                .expect("Expected space after key");

            let key = String::from_utf8_lossy(&raw[pos..space_idx]).into_owned();

            // Find the end of the value, including continuation lines
            let mut end = space_idx;
            loop {
                let newline_idx = raw[end + 1..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .map(|i| i + end + 1)
                    .expect("Expected newline");

                if raw.get(newline_idx + 1) != Some(&b' ') {
                    end = newline_idx;
                    break;
                }
                end = newline_idx;
            }

            // Extract and de-indent continuation lines
            let value_raw = &raw[space_idx + 1..=end];
            let value = String::from_utf8_lossy(value_raw)
                .replace("\n ", "\n")
                .to_string();

            result
                .entry(Some(key))
                .and_modify(|v| v.push(value.clone()))
                .or_insert_with(|| vec![value]);

            pos = end + 1;
        }

        result
    }
}
