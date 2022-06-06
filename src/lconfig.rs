use pwhash::bcrypt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;

use directories_next::ProjectDirs;

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum TextScrollOrientation {
    ScrollVertical,
    ScrollHorizontal,
}

const DEFAULT_HTTP_PORT: u16 = 3000;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub version: u64,
    pub http_port: u16,
    pub disp_text: String,
    pub disp_scrollspeed: i16,
    pub disp_orientation: TextScrollOrientation,
    pub disp_textcolor: String,
    pub disp_backgroundcolor: String,
    pub disp_hmargin: u16,
    pub disp_vmargin: u16,
    pub disp_fontsize: u16,
    pub disp_fullscreen: bool,
    pub encrypted_admin_password: String,
}

// Global data, will be wrapped on an Arc<Mutex<>>
pub struct RuntimeGlobalData {
    pub runtime_data_dir: PathBuf,
    pub html_dir: PathBuf,
    pub upload_dir: PathBuf,
    pub cfg: Config,
}

impl Config {
    pub fn new() -> Config {
        Config {
            version: 0,
            http_port: DEFAULT_HTTP_PORT,
            disp_text: String::from("Text"),
            disp_scrollspeed: 1,
            disp_orientation: TextScrollOrientation::ScrollVertical,
            disp_textcolor: String::from("#ffffff"),
            disp_backgroundcolor: String::from("#202020"),
            disp_hmargin: 10,
            disp_vmargin: 10,
            disp_fontsize: 18,
            disp_fullscreen: false,
            encrypted_admin_password: String::from(""),
        }
    }

    pub fn get_config_file_name(create_dir: bool) -> PathBuf {
        let mut pb = PathBuf::new();
        match ProjectDirs::from(
            crate::APP_NAME_QUALIFIER,
            crate::APP_NAME_ORGANIZATION,
            crate::APP_NAME_APPLICATION,
        ) {
            Some(proj_dirs) => pb.push(proj_dirs.config_dir()),
            None => pb.push("."),
        };
        if create_dir && !pb.exists() {
            // Try to create pb directory if it does not exists
            let _ = std::fs::create_dir_all(pb.clone());
        }

        let mut cf_filename: String = crate::APP_NAME_APPLICATION.to_owned();
        cf_filename.push_str(".json");
        pb.join(&cf_filename)
    }

    pub async fn load() -> Config {
        let filepath = Config::get_config_file_name(false);
        let default_config = r#"
			{
			}
		"#
        .to_owned();

        println!("Loading {}", filepath.display());
        let contents = match fs::read_to_string(&filepath).await {
            Ok(s) => s,
            Err(_) => default_config.clone(),
        };

        let jconf = match serde_json::from_str::<Value>(&contents) {
            Ok(parsed) => parsed,
            Err(_) => json!(default_config.clone()),
        };

        let mut cf = Config::new();
        cf.set_partial_cfg(&jconf);
        cf.version = 1;

        cf
    }

    /*
    pub async fn set_http_port(&mut self, new_http_port: u16) -> tokio::io::Result<()> {
        self.http_port = new_http_port;
        self.save().await ?;
        Ok(())
    }
    */

    pub fn set_partial_cfg(&mut self, cfg: &Value) {
        println!("set_partial_cfg {:?}", cfg);

        if let Some(v) = cfg.get("http_port").and_then(Value::as_u64) {
            self.http_port = v as u16;
        }
        if let Some(v) = cfg.get("disp_text").and_then(Value::as_str) {
            self.disp_text = v.to_string();
        }
        if let Some(v) = cfg.get("disp_textcolor").and_then(Value::as_str) {
            self.disp_textcolor = v.to_string();
        }
        if let Some(v) = cfg.get("disp_backgroundcolor").and_then(Value::as_str) {
            self.disp_backgroundcolor = v.to_string();
        }
        if let Some(v) = cfg.get("disp_vmargin").and_then(Value::as_u64) {
            self.disp_vmargin = v as u16;
        }
        if let Some(v) = cfg.get("disp_hmargin").and_then(Value::as_u64) {
            self.disp_hmargin = v as u16;
        }
        if let Some(v) = cfg.get("disp_fontsize").and_then(Value::as_u64) {
            self.disp_fontsize = v as u16;
        }
        if let Some(v) = cfg.get("disp_scrollspeed").and_then(Value::as_i64) {
            self.disp_scrollspeed = v as i16;
            if self.disp_scrollspeed < -50 {
                self.disp_scrollspeed = -50;
            }
            if self.disp_scrollspeed > 50 {
                self.disp_scrollspeed = 50;
            }
        }
        if let Some(v) = cfg.get("disp_fullscreen").and_then(Value::as_bool) {
            self.disp_fullscreen = v;
        }
        if let Some(v) = cfg.get("disp_orientation") {
            match TextScrollOrientation::deserialize(v) {
                Ok(vv) => self.disp_orientation = vv,
                Err(_e) => {}
            }
        }
        if let Some(v) = cfg.get("encrypted_admin_password").and_then(Value::as_str) {
            self.encrypted_admin_password = v.to_string();
        }

        /* Create a default admin password "admin" */
        if self.encrypted_admin_password.is_empty() {
            self.encrypted_admin_password = match bcrypt::hash("admin") {
                Ok(s) => s,
                Err(_e) => String::from(""),
            };
        }

        self.version += 1;
    }

    pub async fn save(&mut self) -> tokio::io::Result<()> {
        let jcfg = json!(self);
        let cfpath = Config::get_config_file_name(true);
        println!("Saving configuration to {:?}", cfpath);
        tokio::fs::write(cfpath, jcfg.to_string()).await?;
        Ok(())
    }
}
