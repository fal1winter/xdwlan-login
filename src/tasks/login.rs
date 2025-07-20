use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use super::{AppEvent, Task};

const NODE_ENV: &str = if cfg!(debug_assertions) {
    "development"
} else {
    "production"
};

const DEFAULT_LOGIN_URL: &str = "https://w.xidian.edu.cn/index_8.html";

pub struct LoginTask {
    username: String,
    password: String,
    domain: String,

    client: reqwest::blocking::Client,
}

impl LoginTask {
    pub fn new(username: String, password: String, domain: String) -> Self {
        LoginTask {
            username,
            password,
            domain,

            client: reqwest::blocking::ClientBuilder::new()
                .no_proxy()
                .timeout(Duration::from_secs(10)) // Set a timeout to avoid blocking for too long.
                .build()
                .unwrap(), // This method only panics if called from within an async runtime.
        }
    }

    pub fn is_online(&self) -> bool {
        match self.client.get("http://baidu.com").send() {
            Ok(resp) => {
                // In both online and offline status, the response should be 200 OK.
                if !resp.status().is_success() {
                    log::debug!("Offline detected via status: {}", resp.status().as_u16());
                    return false;
                }

                let headers = resp.headers();
                if let Some(server) = headers.get("Server") {
                    let server = server.to_str().unwrap_or("");
                    if server.contains("NetEngine Server") {
                        // If the server is NetEngine Server, it comes from the login portal, which means you are offline.
                        // Because Baidu uses Apache.
                        log::debug!("Offline detected via server header: {:?}", server);
                        return false;
                    }
                }

                let body_text = resp.text();
                if body_text.is_err() {
                    log::debug!("Offline detected via decoding body: {:?}", body_text);
                    return false;
                }
                let body_text = body_text.unwrap();
                if body_text.contains("w.xidian.edu.cn") {
                    // If the body contains "w.xidian.edu.cn", it means you are offline.
                    log::debug!("Offline detected via body content.");
                    return false;
                }

                // In any other cases, it is considered online.
                log::debug!("All offline detections passed. You are online.");
                return true;
            }
            Err(err) => {
                log::debug!("Offline detected via error: {:?}", err);
                return false;
            }
        }
    }

    fn get_login_url(&self) -> anyhow::Result<String> {
        // When you were offline, you will be redirct to the login page.
        // Sometimes, the redirection will fail, so we try at most 5 times.
        for _ in 0..5 {
            let resp = self.client.get("http://www.baidu.com").send()?;
            let content = resp.text()?;
            if content.contains("w.xidian.edu.cn") {
                let re = regex::Regex::new(
                    r#"(?m)action="(?P<url>https://w\.xidian\.edu\.cn[a-zA-Z0-9./_]+)""#,
                )?;
                if let Some(cap) = re.captures(&content) {
                    return Ok(cap["url"].to_string());
                }
            }
        }

        Err(anyhow::anyhow!("Login url not found.").into())
    }

    /// Open a browser and login to the network.
    pub fn login(&self) -> anyhow::Result<()> {
        let url = match self.get_login_url() {
            Ok(url) => {
                log::info!("Got login url: {}", url);
                url
            }
            Err(e) => {
                log::warn!("Failed to get login url: {}", e);
                log::warn!("Try using default login url: {}", DEFAULT_LOGIN_URL);
                DEFAULT_LOGIN_URL.to_string()
            }
        };

        let program_folder = crate::utils::get_program_folder();

        #[cfg(target_os = "windows")]
        let login_exe = program_folder.join("xdwlan-login-worker.exe");

        #[cfg(target_os = "linux")]
        let login_exe = program_folder.join("xdwlan-login-worker");

        let output = std::process::Command::new(login_exe)
            .env("NODE_ENV", NODE_ENV)
            .env("XDWLAN_LOGIN_URL", &url)
            .env("XDWLAN_USERNAME", &self.username)
            .env("XDWLAN_PASSWORD", &self.password)
            .env("XDWLAN_DOMAIN", &self.domain)
            .output()?;

        if output.status.success() {
            return Ok(());
        }

        Err(anyhow::anyhow!(
            "Login process exited with code {}\nOutput:{}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

impl Task for LoginTask {
    fn run(&self, _sender: Sender<AppEvent>, receiver: Receiver<AppEvent>) -> anyhow::Result<()> {
        log::debug!("Login task started.");
        log::debug!(
            "Use username: {} and password: {}",
            self.username,
            self.password
        );

        // Sleep seconds and wake up when receive a message.
        let should_quit = |seconds: u64| {
            if let Ok(AppEvent::Quit) = receiver.recv_timeout(Duration::from_secs(seconds)) {
                return true;
            } else {
                return false;
            }
        };

        let simulate = || {
            log::info!("You are offline now.");

            loop {
                if let Err(e) = self.login() {
                    log::error!("{}", e);
                }

                // Wait a second for network to be ready.
                if should_quit(1) {
                    return;
                }

                if self.is_online() {
                    log::info!("Login successfully.");
                    break;
                }

                // Hang up for 5 seconds for next login attempt to avoid being banned.
                if should_quit(5) {
                    return;
                }
            }
        };

        // Check the network status at first.
        if self.is_online() {
            log::info!("You are already online.");
        } else {
            simulate();
        }

        loop {
            if should_quit(60) {
                return Ok(());
            }

            if !self.is_online() {
                simulate();
            }
        }
    }
}
