use roxmltree::Document;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct PcrManager {
    curves: BTreeMap<u64, Vec<(f64, f64)>>,
    packet_size: usize,
}

impl PcrManager {
    pub fn load(uri: &str) -> Result<Self, String> {
        let path = uri.strip_prefix("file://").unwrap_or(uri);
        let content = std::fs::read_to_string(path)
            .map_err(|error| format!("failed to read PCR curve {path}: {error}"))?;
        let document = Document::parse(&content)
            .map_err(|error| format!("failed to parse PCR curve {path}: {error}"))?;
        let root = document.root_element();
        if !root.has_tag_name("tdmabasemodel-pcr") {
            return Err("PCR curve has an invalid document root".to_string());
        }
        let packet_size = root
            .attribute("packetsize")
            .unwrap_or("0")
            .parse::<usize>()
            .map_err(|_| "PCR curve has an invalid packetsize".to_string())?;
        let mut curves = BTreeMap::new();
        for table in root.children().filter(|node| node.has_tag_name("datarate")) {
            let rate = parse_rate(
                table
                    .attribute("bps")
                    .ok_or_else(|| "PCR datarate is missing bps".to_string())?,
            )
            .ok_or_else(|| "PCR datarate has an invalid bps".to_string())?;
            let mut points = Vec::new();
            for entry in table.children().filter(|node| node.has_tag_name("entry")) {
                let sinr = entry
                    .attribute("sinr")
                    .ok_or_else(|| "PCR entry is missing sinr".to_string())?
                    .parse::<f64>()
                    .map_err(|_| "PCR entry has an invalid sinr".to_string())?;
                let por = entry
                    .attribute("por")
                    .ok_or_else(|| "PCR entry is missing por".to_string())?
                    .parse::<f64>()
                    .map_err(|_| "PCR entry has an invalid por".to_string())?;
                if !sinr.is_finite() || !por.is_finite() || !(0.0..=100.0).contains(&por) {
                    return Err("PCR entry is outside its valid range".to_string());
                }
                if points.last().is_some_and(|(previous, _)| sinr <= *previous) {
                    return Err("PCR SINR values must be strictly increasing".to_string());
                }
                points.push((sinr, por / 100.0));
            }
            if points.is_empty() || curves.insert(rate, points).is_some() {
                return Err("PCR curve has an empty or duplicate datarate".to_string());
            }
        }
        if curves.is_empty() {
            return Err("PCR curve has no datarates".to_string());
        }
        Ok(Self {
            curves,
            packet_size,
        })
    }

    pub fn probability(&self, rate: u64, sinr: f64, packet_length: usize) -> f64 {
        let Some(points) = self.curves.get(&rate) else {
            return 0.0;
        };
        let probability = match points.binary_search_by(|(value, _)| value.total_cmp(&sinr)) {
            Ok(index) => points[index].1,
            Err(0) => 0.0,
            Err(index) if index == points.len() => 1.0,
            Err(index) => {
                let (x0, y0) = points[index - 1];
                let (x1, y1) = points[index];
                y0 + (sinr - x0) / (x1 - x0) * (y1 - y0)
            }
        };
        if self.packet_size == 0 {
            probability
        } else {
            probability.powf(packet_length as f64 / self.packet_size as f64)
        }
        .clamp(0.0, 1.0)
    }
}

pub fn parse_rate(value: &str) -> Option<u64> {
    let (number, multiplier) = match value.as_bytes().last().copied() {
        Some(b'K' | b'k') => (&value[..value.len() - 1], 1_000.0),
        Some(b'M' | b'm') => (&value[..value.len() - 1], 1_000_000.0),
        Some(b'G' | b'g') => (&value[..value.len() - 1], 1_000_000_000.0),
        _ => (value, 1.0),
    };
    let number = number.parse::<f64>().ok()?;
    let scaled = number * multiplier;
    (number > 0.0 && scaled.is_finite() && scaled <= u64::MAX as f64)
        .then_some(scaled.round() as u64)
}
