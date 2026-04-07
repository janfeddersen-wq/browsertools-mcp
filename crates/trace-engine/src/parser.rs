//! Chrome trace JSON parser.
//!
//! Parses Chrome trace format (array of events or {traceEvents: [...]}).

use std::io::Read;

use crate::model::TraceEvent;

/// Parse a Chrome trace from a JSON byte slice.
pub fn parse_trace(data: &[u8]) -> anyhow::Result<Vec<TraceEvent>> {
    // Try to decompress if gzipped.
    let json_data = if data.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = flate2::read::GzDecoder::new(data);
        let mut decompressed = String::new();
        decoder.read_to_string(&mut decompressed)?;
        decompressed
    } else {
        String::from_utf8_lossy(data).into_owned()
    };

    let value: serde_json::Value = serde_json::from_str(&json_data)?;

    // Handle both formats: bare array or {traceEvents: [...]}
    let events_value = if let Some(trace_events) = value.get("traceEvents") {
        trace_events.clone()
    } else if value.is_array() {
        value
    } else {
        anyhow::bail!("Invalid trace format: expected array or object with traceEvents");
    };

    let events: Vec<TraceEvent> = serde_json::from_value(events_value)?;
    Ok(events)
}

/// Save trace events to a gzipped JSON file.
pub fn save_trace_gz(events: &[TraceEvent], path: &std::path::Path) -> anyhow::Result<()> {
    use flate2::Compression;
    use flate2::write::GzEncoder;

    let file = std::fs::File::create(path)?;
    let mut encoder = GzEncoder::new(file, Compression::default());
    serde_json::to_writer(&mut encoder, events)?;
    encoder.finish()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_array_format() {
        let json = r#"[{"name":"test","cat":"","ph":"X","ts":0,"pid":1,"tid":1}]"#;
        let events = parse_trace(json.as_bytes()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].name, "test");
    }

    #[test]
    fn test_parse_object_format() {
        let json = r#"{"traceEvents":[{"name":"test","cat":"","ph":"X","ts":0,"pid":1,"tid":1}]}"#;
        let events = parse_trace(json.as_bytes()).unwrap();
        assert_eq!(events.len(), 1);
    }
}
