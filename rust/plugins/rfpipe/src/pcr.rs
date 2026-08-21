use roxmltree::Document;

#[derive(Default)]
pub struct PcrCurve {
    points: Vec<(f64, f64)>,
    packet_size: usize,
}

impl PcrCurve {
    pub fn load(uri: &str) -> Result<Self, String> {
        let path = uri.strip_prefix("file://").unwrap_or(uri);
        let content = std::fs::read_to_string(path)
            .map_err(|error| format!("failed to read PCR curve {path}: {error}"))?;
        let document = Document::parse(&content)
            .map_err(|error| format!("failed to parse PCR curve {path}: {error}"))?;
        let root = document.root_element();
        if !root.has_tag_name("pcr") {
            return Err("PCR curve has an invalid document root".to_string());
        }
        let mut tables = root.children().filter(|node| node.has_tag_name("table"));
        let table = tables
            .next()
            .ok_or_else(|| "PCR curve has no table".to_string())?;
        if tables.next().is_some() {
            return Err("PCR curve has multiple tables".to_string());
        }
        let packet_size = table
            .attribute("pktsize")
            .unwrap_or("0")
            .parse::<usize>()
            .map_err(|_| "PCR table has an invalid pktsize".to_string())?;
        let mut points = Vec::new();
        for row in table.children().filter(|node| node.has_tag_name("row")) {
            let sinr = row
                .attribute("sinr")
                .ok_or_else(|| "PCR row is missing sinr".to_string())?
                .parse::<f64>()
                .map_err(|_| "PCR row has an invalid sinr".to_string())?;
            let percent = row
                .attribute("por")
                .ok_or_else(|| "PCR row is missing por".to_string())?
                .parse::<f64>()
                .map_err(|_| "PCR row has an invalid por".to_string())?;
            if !sinr.is_finite() || !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
                return Err("PCR row is outside its valid range".to_string());
            }
            if points.last().is_some_and(|(previous, _)| sinr <= *previous) {
                return Err("PCR SINR values must be strictly increasing".to_string());
            }
            points.push((sinr, percent / 100.0));
        }
        if points.is_empty() {
            return Err("PCR curve has no rows".to_string());
        }
        Ok(Self {
            points,
            packet_size,
        })
    }

    pub fn probability(&self, sinr: f64, packet_length: usize) -> f64 {
        let probability = match self
            .points
            .binary_search_by(|(value, _)| value.total_cmp(&sinr))
        {
            Ok(index) => self.points[index].1,
            Err(0) => 0.0,
            Err(index) if index == self.points.len() => 1.0,
            Err(index) => {
                let (x0, y0) = self.points[index - 1];
                let (x1, y1) = self.points[index];
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_curve_fails_without_panicking() {
        assert!(PcrCurve::load("/path/that/does/not/exist").is_err());
    }
}
