use std::process::Command;

#[derive(Debug)]
struct NetworkAdapter {
    name: String,
    ipv4: Option<String>,
    ipv6: Option<String>,
    subnet_mask: Option<String>,
    default_gateway: Option<String>,
    dns_suffix: Option<String>,
    media_state: Option<String>,
}

pub fn get_ip_address(args: Vec<&str>) {
    match Command::new("ipconfig").output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);

            let mut adapters: Vec<NetworkAdapter> = Vec::new();

            let mut current_adapter: Option<NetworkAdapter> = None;

            for line in stdout.lines() {
                let line = line.trim();

                if line.is_empty() {
                    continue;
                }

                // Detect adapter section
                if line.ends_with(":") && !line.contains(".") {
                    // Save previous adapter
                    if let Some(adapter) = current_adapter.take() {
                        adapters.push(adapter);
                    }

                    current_adapter = Some(NetworkAdapter {
                        name: line.replace(":", ""),
                        ipv4: None,
                        ipv6: None,
                        subnet_mask: None,
                        default_gateway: None,
                        dns_suffix: None,
                        media_state: None,
                    });

                    continue;
                }

                // Parse key-value lines
                if line.contains(":") {
                    let parts: Vec<&str> = line.splitn(2, ":").collect();

                    if parts.len() != 2 {
                        continue;
                    }

                    let key = parts[0].replace(".", "").trim().to_string();

                    let value = parts[1].trim().to_string();

                    if let Some(adapter) = current_adapter.as_mut() {
                        match key.as_str() {
                            "IPv4 Address" => {
                                adapter.ipv4 = Some(value);
                            }

                            "Link-local IPv6 Address" => {
                                adapter.ipv6 = Some(value);
                            }

                            "Subnet Mask" => {
                                adapter.subnet_mask = Some(value);
                            }

                            "Default Gateway" => {
                                adapter.default_gateway = Some(value);
                            }

                            "Connection-specific DNS Suffix" => {
                                adapter.dns_suffix = Some(value);
                            }

                            "Media State" => {
                                adapter.media_state = Some(value);
                            }

                            _ => {}
                        }
                    }
                }
            }

            // Push last adapter
            if let Some(adapter) = current_adapter {
                adapters.push(adapter);
            }

            // Print all adapters
            for adapter in adapters {
                println!("{:#?}", adapter);
            }
        }

        Err(e) => {
            println!("Error running command: {}", e);
        }
    }
}
