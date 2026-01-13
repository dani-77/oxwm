use std::fs;
use std::io::{self, BufRead};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct WifiInfo {
    pub interface: String,
    pub ssid: String,
    pub signal_strength: i32,
    pub frequency: String,
    pub tx_bytes: u64,
    pub rx_bytes: u64,
    pub tx_packets: u64,
    pub rx_packets: u64,
}

impl WifiInfo {
    /// Cria uma nova instância de WifiInfo
    pub fn new() -> Result<Self, io::Error> {
        let interface = Self::get_wireless_interface()?;
        let ssid = Self::get_ssid(&interface)?;
        let signal_strength = Self::get_signal_strength(&interface)?;
        let frequency = Self::get_frequency(&interface)?;
        let (tx_bytes, rx_bytes, tx_packets, rx_packets) = Self::get_network_stats(&interface)?;

        Ok(WifiInfo {
            interface,
            ssid,
            signal_strength,
            frequency,
            tx_bytes,
            rx_bytes,
            tx_packets,
            rx_packets,
        })
    }

    /// Detecta a interface wireless disponível
    fn get_wireless_interface() -> Result<String, io::Error> {
        let wireless_path = Path::new("/proc/net/wireless");
        if !wireless_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "No wireless interface found",
            ));
        }

        let file = fs::File::open(wireless_path)?;
        let reader = io::BufReader::new(file);

        for line in reader.lines().skip(2) {
            let line = line?;
            if let Some(interface) = line.split(':').next() {
                return Ok(interface.trim().to_string());
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No active wireless interface",
        ))
    }

    /// Obtém o SSID da rede conectada
    fn get_ssid(interface: &str) -> Result<String, io::Error> {
        let output = std::process::Command::new("iw")
            .args(&["dev", interface, "link"])
            .output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        
        for line in output_str.lines() {
            if line.contains("SSID:") {
                if let Some(ssid) = line.split("SSID:").nth(1) {
                    return Ok(ssid.trim().to_string());
                }
            }
        }

        Ok("Not Connected".to_string())
    }

    /// Obtém a força do sinal em dBm
    fn get_signal_strength(interface: &str) -> Result<i32, io::Error> {
        let output = std::process::Command::new("iw")
            .args(&["dev", interface, "link"])
            .output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        
        for line in output_str.lines() {
            if line.contains("signal:") {
                if let Some(signal_part) = line.split("signal:").nth(1) {
                    if let Some(dbm_str) = signal_part.trim().split_whitespace().next() {
                        if let Ok(signal) = dbm_str.parse::<i32>() {
                            return Ok(signal);
                        }
                    }
                }
            }
        }

        Ok(0)
    }

    /// Obtém a frequência da rede
    fn get_frequency(interface: &str) -> Result<String, io::Error> {
        let output = std::process::Command::new("iw")
            .args(&["dev", interface, "link"])
            .output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        
        for line in output_str.lines() {
            if line.contains("freq:") {
                if let Some(freq) = line.split("freq:").nth(1) {
                    return Ok(freq.trim().split_whitespace().next().unwrap_or("Unknown").to_string() + " MHz");
                }
            }
        }

        Ok("Unknown".to_string())
    }

    /// Obtém estatísticas de rede (bytes e pacotes transmitidos/recebidos)
    fn get_network_stats(interface: &str) -> Result<(u64, u64, u64, u64), io::Error> {
        let tx_bytes_path = format!("/sys/class/net/{}/statistics/tx_bytes", interface);
        let rx_bytes_path = format!("/sys/class/net/{}/statistics/rx_bytes", interface);
        let tx_packets_path = format!("/sys/class/net/{}/statistics/tx_packets", interface);
        let rx_packets_path = format!("/sys/class/net/{}/statistics/rx_packets", interface);

        let tx_bytes = fs::read_to_string(&tx_bytes_path)?
            .trim()
            .parse::<u64>()
            .unwrap_or(0);
        let rx_bytes = fs::read_to_string(&rx_bytes_path)?
            .trim()
            .parse::<u64>()
            .unwrap_or(0);
        let tx_packets = fs::read_to_string(&tx_packets_path)?
            .trim()
            .parse::<u64>()
            .unwrap_or(0);
        let rx_packets = fs::read_to_string(&rx_packets_path)?
            .trim()
            .parse::<u64>()
            .unwrap_or(0);

        Ok((tx_bytes, rx_bytes, tx_packets, rx_packets))
    }

    /// Retorna uma string formatada com informações de WiFi
    pub fn format_display(&self) -> String {
        let signal_bars = Self::signal_to_bars(self.signal_strength);
        format!(
            "WiFi: {} {} ({} dBm) | {} | ↑{} ↓{}",
            self.ssid,
            signal_bars,
            self.signal_strength,
            self.frequency,
            Self::bytes_to_human(self.tx_bytes),
            Self::bytes_to_human(self.rx_bytes)
        )
    }

    /// Converte força do sinal em barras visuais
    fn signal_to_bars(signal: i32) -> &'static str {
        match signal {
            s if s >= -50 => "▂▄▆█",
            s if s >= -60 => "▂▄▆_",
            s if s >= -70 => "▂▄__",
            s if s >= -80 => "▂___",
            _ => "____",
        }
    }

    /// Converte bytes para formato legível (KB, MB, GB)
    fn bytes_to_human(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2}GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2}MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2}KB", bytes as f64 / KB as f64)
        } else {
            format!("{}B", bytes)
        }
    }

    /// Calcula a taxa de transferência entre duas leituras
    pub fn calculate_rate(&self, previous: &WifiInfo) -> (f64, f64) {
        let tx_rate = (self.tx_bytes - previous.tx_bytes) as f64;
        let rx_rate = (self.rx_bytes - previous.rx_bytes) as f64;
        (tx_rate, rx_rate)
    }
}

impl Default for WifiInfo {
    fn default() -> Self {
        Self {
            interface: String::new(),
            ssid: "Disconnected".to_string(),
            signal_strength: 0,
            frequency: "N/A".to_string(),
            tx_bytes: 0,
            rx_bytes: 0,
            tx_packets: 0,
            rx_packets: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_to_human() {
        assert_eq!(WifiInfo::bytes_to_human(500), "500B");
        assert_eq!(WifiInfo::bytes_to_human(1024), "1.00KB");
        assert_eq!(WifiInfo::bytes_to_human(1048576), "1.00MB");
        assert_eq!(WifiInfo::bytes_to_human(1073741824), "1.00GB");
    }

    #[test]
    fn test_signal_bars() {
        assert_eq!(WifiInfo::signal_to_bars(-45), "▂▄▆█");
        assert_eq!(WifiInfo::signal_to_bars(-65), "▂▄__");
        assert_eq!(WifiInfo::signal_to_bars(-85), "____");
    }
}
