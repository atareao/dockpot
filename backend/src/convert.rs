use std::collections::HashMap;

use serde::Serialize;

/// Parse a `docker run ...` command line and produce a compose.yaml service definition
pub fn docker_run_to_compose(cmd: &str, service_name: &str) -> Result<String, String> {
    let args = shlex::split(cmd).ok_or_else(|| "Failed to parse command line".to_string())?;

    if args.is_empty() {
        return Err("Empty command".to_string());
    }

    // Find "run" position
    let run_pos = args.iter().position(|a| a == "run")
        .ok_or_else(|| "Not a docker run command (missing 'run')".to_string())?;

    let docker_args = &args[run_pos + 1..];
    if docker_args.is_empty() {
        return Err("No arguments after 'run'".to_string());
    }

    let mut service = ComposeService::new(service_name);

    let mut i = 0;
    let mut image_found = false;

    while i < docker_args.len() {
        let arg = &docker_args[i];
        let next = docker_args.get(i + 1);

        match arg.as_str() {
            "-d" | "--detach" => {
                // Detached mode — default for compose
                i += 1;
                continue;
            }
            "--name" => {
                if let Some(name) = next {
                    service.container_name = Some(name.clone());
                    i += 2;
                } else {
                    return Err("--name requires a value".to_string());
                }
                continue;
            }
            "--hostname" => {
                if let Some(hostname) = next {
                    service.hostname = Some(hostname.clone());
                    i += 2;
                } else {
                    return Err("--hostname requires a value".to_string());
                }
                continue;
            }
            "--restart" => {
                if let Some(policy) = next {
                    service.restart = Some(policy.clone());
                    i += 2;
                } else {
                    return Err("--restart requires a value".to_string());
                }
                continue;
            }
            "--network" => {
                if let Some(net) = next {
                    service.networks.push(net.clone());
                    i += 2;
                } else {
                    return Err("--network requires a value".to_string());
                }
                continue;
            }
            "--entrypoint" => {
                if let Some(ep) = next {
                    service.entrypoint = Some(ep.clone());
                    i += 2;
                } else {
                    return Err("--entrypoint requires a value".to_string());
                }
                continue;
            }
            "--user" => {
                if let Some(user) = next {
                    service.user = Some(user.clone());
                    i += 2;
                } else {
                    return Err("--user requires a value".to_string());
                }
                continue;
            }
            "--workdir" | "-w" => {
                if let Some(dir) = next {
                    service.working_dir = Some(dir.clone());
                    i += 2;
                } else {
                    return Err("--workdir requires a value".to_string());
                }
                continue;
            }
            "-p" | "--publish" => {
                if let Some(port) = next {
                    service.ports.push(port.clone());
                    i += 2;
                } else {
                    return Err("-p requires a value".to_string());
                }
                continue;
            }
            "-v" | "--volume" => {
                if let Some(vol) = next {
                    service.volumes.push(vol.clone());
                    i += 2;
                } else {
                    return Err("-v requires a value".to_string());
                }
                continue;
            }
            "-e" | "--env" => {
                if let Some(env) = next {
                    if let Some((k, v)) = env.split_once('=') {
                        service.environment.insert(k.to_string(), v.to_string());
                    } else {
                        // Just key — use empty or look up in host env
                        service.environment_keys.push(env.clone());
                    }
                    i += 2;
                } else {
                    return Err("-e requires a value".to_string());
                }
                continue;
            }
            "-l" | "--label" => {
                if let Some(label) = next {
                    if let Some((k, v)) = label.split_once('=') {
                        service.labels.insert(k.to_string(), v.to_string());
                    }
                    i += 2;
                } else {
                    return Err("--label requires a value".to_string());
                }
                continue;
            }
            "-m" | "--memory" => {
                if let Some(mem) = next {
                    service.memory = Some(format!("{}m", parse_memory_mb(mem)?));
                    i += 2;
                } else {
                    return Err("--memory requires a value".to_string());
                }
                continue;
            }
            "--cpus" => {
                if let Some(cpus) = next {
                    service.cpus = Some(cpus.clone());
                    i += 2;
                } else {
                    return Err("--cpus requires a value".to_string());
                }
                continue;
            }
            "--cap-add" | "--cap-drop" | "--privileged" | "--security-opt" | "--shm-size" | "--tmpfs" => {
                // Pass through: skip value args
                if arg == "--privileged" {
                    service.privileged = true;
                    i += 1;
                } else {
                    i += 2; // skip flag + value
                }
                continue;
            }
            "--env-file" => {
                if let Some(file) = next {
                    service.env_file = Some(file.clone());
                    i += 2;
                } else {
                    return Err("--env-file requires a value".to_string());
                }
                continue;
            }
            "--device" => {
                if let Some(dev) = next {
                    service.devices.push(dev.clone());
                    i += 2;
                } else {
                    return Err("--device requires a value".to_string());
                }
                continue;
            }
            "--sysctl" => {
                if let Some(ctl) = next {
                    if let Some((k, v)) = ctl.split_once('=') {
                        service.sysctls.insert(k.to_string(), v.to_string());
                    }
                    i += 2;
                } else {
                    return Err("--sysctl requires a value".to_string());
                }
                continue;
            }
            "--dns" => {
                if let Some(dns) = next {
                    service.dns.push(dns.clone());
                    i += 2;
                } else {
                    return Err("--dns requires a value".to_string());
                }
                continue;
            }
            // Anything else starting with -- or - is a flag we skip (with or without value)
            s if s.starts_with("--") || s.starts_with('-') && s.len() == 2 => {
                // Unknown flag with possible value arg
                if let Some(next_val) = next {
                    if !next_val.starts_with('-') {
                        i += 2;
                        continue;
                    }
                }
                i += 1;
                continue;
            }
            // The image name — anything that doesn't start with -
            s => {
                if !image_found {
                    service.image = s.to_string();
                    image_found = true;
                    // Remaining args after image are command
                    if i + 1 < docker_args.len() {
                        service.command = docker_args[i + 1..].to_vec();
                    }
                    i = docker_args.len();
                } else {
                    i += 1;
                }
                continue;
            }
        }
    }

    if !image_found {
        return Err("No Docker image specified".to_string());
    }

    Ok(service.to_yaml())
}

fn parse_memory_mb(s: &str) -> Result<u64, String> {
    let s = s.trim().to_lowercase();
    let (num_str, _) = s.split_at(
        s.find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(s.len()),
    );
    let num: f64 = num_str.parse().map_err(|_| format!("Invalid memory value: {}", s))?;

    if s.ends_with('g') || s.ends_with("gb") {
        Ok((num * 1024.0) as u64)
    } else if s.ends_with('m') || s.ends_with("mb") {
        Ok(num as u64)
    } else if s.ends_with('k') || s.ends_with("kb") {
        Ok((num / 1024.0).max(1.0) as u64)
    } else {
        // Default: bytes → MB
        Ok((num / 1024.0 / 1024.0).max(1.0) as u64)
    }
}

// ───── Compose Service Builder ─────

#[derive(Debug, Serialize)]
struct ComposeService {
    service_name: String,
    image: String,
    container_name: Option<String>,
    hostname: Option<String>,
    restart: Option<String>,
    networks: Vec<String>,
    ports: Vec<String>,
    volumes: Vec<String>,
    environment: HashMap<String, String>,
    environment_keys: Vec<String>,
    labels: HashMap<String, String>,
    devices: Vec<String>,
    dns: Vec<String>,
    sysctls: HashMap<String, String>,
    entrypoint: Option<String>,
    user: Option<String>,
    working_dir: Option<String>,
    memory: Option<String>,
    cpus: Option<String>,
    privileged: bool,
    env_file: Option<String>,
    command: Vec<String>,
}

impl ComposeService {
    fn new(name: &str) -> Self {
        Self {
            service_name: name.to_string(),
            image: String::new(),
            container_name: None,
            hostname: None,
            restart: None,
            networks: Vec::new(),
            ports: Vec::new(),
            volumes: Vec::new(),
            environment: HashMap::new(),
            environment_keys: Vec::new(),
            labels: HashMap::new(),
            devices: Vec::new(),
            dns: Vec::new(),
            sysctls: HashMap::new(),
            entrypoint: None,
            user: None,
            working_dir: None,
            memory: None,
            cpus: None,
            privileged: false,
            env_file: None,
            command: Vec::new(),
        }
    }

    fn to_yaml(&self) -> String {
        let mut yaml = String::new();
        yaml.push_str(&format!("services:\n  {}:\n", self.service_name));

        yaml.push_str(&format!("    image: {}\n", self.image));

        if let Some(ref name) = self.container_name {
            yaml.push_str(&format!("    container_name: {}\n", name));
        }
        if let Some(ref hostname) = self.hostname {
            yaml.push_str(&format!("    hostname: {}\n", hostname));
        }
        if let Some(ref restart) = self.restart {
            yaml.push_str(&format!("    restart: {}\n", restart));
        }
        if let Some(ref user) = self.user {
            yaml.push_str(&format!("    user: {}\n", user));
        }
        if let Some(ref dir) = self.working_dir {
            yaml.push_str(&format!("    working_dir: {}\n", dir));
        }
        if self.privileged {
            yaml.push_str("    privileged: true\n");
        }
        if let Some(ref mem) = self.memory {
            yaml.push_str(&format!("    mem_limit: {}\n", mem));
        }
        if let Some(ref cpus) = self.cpus {
            yaml.push_str(&format!("    cpus: {}\n", cpus));
        }
        if let Some(ref ep) = self.entrypoint {
            yaml.push_str(&format!("    entrypoint: {}\n", ep));
        }
        if let Some(ref file) = self.env_file {
            yaml.push_str(&format!("    env_file: {}\n", file));
        }

        if !self.ports.is_empty() {
            yaml.push_str("    ports:\n");
            for p in &self.ports {
                yaml.push_str(&format!("      - {}\n", p));
            }
        }

        if !self.volumes.is_empty() {
            yaml.push_str("    volumes:\n");
            for v in &self.volumes {
                yaml.push_str(&format!("      - {}\n", v));
            }
        }

        if !self.devices.is_empty() {
            yaml.push_str("    devices:\n");
            for d in &self.devices {
                yaml.push_str(&format!("      - {}\n", d));
            }
        }

        if !self.dns.is_empty() {
            yaml.push_str("    dns:\n");
            for d in &self.dns {
                yaml.push_str(&format!("      - {}\n", d));
            }
        }

        if !self.environment.is_empty() || !self.environment_keys.is_empty() {
            yaml.push_str("    environment:\n");
            for (k, v) in &self.environment {
                yaml.push_str(&format!("      {}: {}\n", k, v));
            }
            for k in &self.environment_keys {
                yaml.push_str(&format!("      {}\n", k));
            }
        }

        if !self.labels.is_empty() {
            yaml.push_str("    labels:\n");
            for (k, v) in &self.labels {
                yaml.push_str(&format!("      {}: {}\n", k, v));
            }
        }

        if !self.sysctls.is_empty() {
            yaml.push_str("    sysctls:\n");
            for (k, v) in &self.sysctls {
                yaml.push_str(&format!("      {}: {}\n", k, v));
            }
        }

        if !self.networks.is_empty() {
            yaml.push_str("    networks:\n");
            for n in &self.networks {
                yaml.push_str(&format!("      - {}\n", n));
            }
        }

        if !self.command.is_empty() {
            yaml.push_str("    command:\n");
            for c in &self.command {
                yaml.push_str(&format!("      - {}\n", c));
            }
        }

        yaml
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_run() {
        let result = docker_run_to_compose(
            "docker run -d --name myapp -p 8080:80 nginx:alpine",
            "myapp",
        ).unwrap();
        assert!(result.contains("image: nginx:alpine"));
        assert!(result.contains("container_name: myapp"));
        assert!(result.contains("ports:"));
        assert!(result.contains("- 8080:80"));
    }

    #[test]
    fn test_with_volumes_env() {
        let result = docker_run_to_compose(
            "docker run -d -v /data:/data -e DB_HOST=localhost -e DB_PORT=5432 postgres:15",
            "db",
        ).unwrap();
        assert!(result.contains("image: postgres:15"));
        assert!(result.contains("- /data:/data"));
        assert!(result.contains("DB_HOST: localhost"));
        assert!(result.contains("DB_PORT: 5432"));
    }

    #[test]
    fn test_with_restart_network() {
        let result = docker_run_to_compose(
            "docker run -d --name api --restart unless-stopped --network mynet --cpus 2 -m 512m myapp:latest",
            "api",
        ).unwrap();
        assert!(result.contains("restart: unless-stopped"));
        assert!(result.contains("networks:"));
        assert!(result.contains("cpus: 2"));
        assert!(result.contains("mem_limit: 512m"));
    }

    #[test]
    fn test_invalid_no_image() {
        let result = docker_run_to_compose("docker run -d", "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_memory_parsing() {
        assert_eq!(parse_memory_mb("1g").unwrap(), 1024);
        assert_eq!(parse_memory_mb("512m").unwrap(), 512);
        assert_eq!(parse_memory_mb("128974848").unwrap(), 123);
    }
}