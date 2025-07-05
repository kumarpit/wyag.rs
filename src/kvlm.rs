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

    pub fn init() -> Self {
        Self {
            data: IndexMap::new(),
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
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

    pub fn get_message(&self) -> &str {
        self.data
            .get(&None)
            .expect("Each kvlm must specify a message")
            .first()
            .expect("Cannot have an empty message")
            .as_str()
    }

    pub fn get_key(&self, key: &str) -> Option<&Vec<String>> {
        self.data.get(&Some(key.to_string()))
    }

    pub fn insert(&mut self, key: &str, value: &str) {
        self.data
            .insert(Some(key.to_string()), vec![value.to_string()]);
    }

    fn parse(raw_data: &[u8]) -> IndexMap<Option<String>, Vec<String>> {
        let mut pos = 0;
        let mut result: IndexMap<Option<String>, Vec<String>> = IndexMap::new();

        while pos < raw_data.len() {
            // Check for the blank line separator (`\n`) -> message body
            if raw_data[pos] == b'\n' {
                let message = String::from_utf8_lossy(&raw_data[pos + 1..]).into_owned();
                result.insert(None, vec![message]);
                break;
            }

            // Parse key
            let space_idx = raw_data[pos..]
                .iter()
                .position(|&b| b == b' ')
                .map(|i| i + pos)
                .expect("Expected space after key");

            let key = String::from_utf8_lossy(&raw_data[pos..space_idx]).into_owned();

            // Find the end of the value, including continuation lines
            let mut end = space_idx;
            loop {
                let newline_idx = raw_data[end + 1..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .map(|i| i + end + 1)
                    .expect("Expected newline");

                end = newline_idx;
                if raw_data.get(newline_idx + 1) != Some(&b' ') {
                    break;
                }
                end = newline_idx;
            }

            // Extract and de-indent continuation lines
            let value_raw_data = &raw_data[space_idx + 1..=end];
            let value = String::from_utf8_lossy(value_raw_data)
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
