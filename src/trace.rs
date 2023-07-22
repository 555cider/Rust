static W3C_TRACEPARENT_VERSION: u8 = 00;
static FLAG_SAMPLED: u8 = 1 << 0; // 00000001

#[derive(Debug, Eq, PartialEq)]
pub struct Traceparent {
    version: u8,     // 8 bit
    trace_id: u128,  // 16 bytes array identifier
    parent_id: u64,  // 8 byte array identifier
    trace_flags: u8, // 8 bit flags
}

impl Traceparent {
    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn trace_id(&self) -> u128 {
        self.trace_id
    }

    pub fn parent_id(&self) -> u64 {
        self.parent_id
    }

    pub fn trace_flags(&self) -> u8 {
        self.trace_flags
    }

    pub fn child(&self, sampled: bool) -> Traceparent {
        Traceparent {
            version: self.version,
            trace_id: self.trace_id,
            parent_id: fastrand::u64(..),
            trace_flags: ((sampled as u8) & self.trace_flags) & FLAG_SAMPLED,
        }
    }

    pub fn sampled(&self) -> bool {
        (FLAG_SAMPLED & self.trace_flags) != 0
    }

    pub fn as_string(&self) -> String {
        format!(
            "{:02x}-{:032x}-{:016x}-{:02x}",
            self.version, self.trace_id, self.parent_id, self.trace_flags
        )
    }

    pub fn as_headervalue(&self) -> http::HeaderValue {
        http::HeaderValue::from_str(self.as_string().as_str())
            .expect("Failed to convert traceparent into header value!")
    }
}

impl std::clone::Clone for Traceparent {
    fn clone(&self) -> Self {
        self.child(self.sampled())
    }
}

impl core::fmt::Display for Traceparent {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:02x}-{:032x}-{:016x}-{:02x}",
            self.version, self.trace_id, self.parent_id, self.trace_flags
        )
    }
}

pub fn new() -> Traceparent {
    Traceparent {
        version: W3C_TRACEPARENT_VERSION,
        trace_id: fastrand::u128(..),
        parent_id: fastrand::u64(..),
        trace_flags: (false as u8) & FLAG_SAMPLED,
    }
}

pub fn extract(headers: &http::HeaderMap) -> Result<Traceparent, std::num::ParseIntError> {
    if headers.get("tracestate").is_none() {
        return Ok(new());
    }

    let traceparent: &str = match headers.get("traceparent") {
        Some(header) => header
            .to_str()
            .expect("Failed to convert ref header value into ref str!"),
        None => return Ok(new()),
    };

    let parts: Vec<&str> = traceparent.split('-').collect();

    Ok(Traceparent {
        version: u8::from_str_radix(parts[0], 16)?,
        trace_id: u128::from_str_radix(parts[1], 16)?,
        parent_id: u64::from_str_radix(parts[2], 16)?,
        trace_flags: u8::from_str_radix(parts[3], 16)?,
    })
}
