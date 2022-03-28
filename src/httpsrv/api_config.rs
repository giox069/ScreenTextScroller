
/*use serde::{Deserialize, Serialize};

use std::path::{Path, PathBuf};
use serde_json::json;
use serde_json::Value;

*/



use tokio::sync::Mutex;
use std::sync::Arc;
use serde_json::json;
use serde_json::Value;


use crate::httpsrv::LpfHttpServerData;
use crate::httpsrv::HTTPAPIError;


pub async fn lapi_config_get(d: Arc<Mutex<LpfHttpServerData>>, _params: Value) -> Result<Value, HTTPAPIError>
{
	let ud = d.lock().await;
	let rgd = ud.runtime_global_data.lock().await;

	let body = json!({
		"http_port": rgd.cfg.http_port,
		"disp_text": rgd.cfg.disp_text,
		"disp_scrollspeed": rgd.cfg.disp_scrollspeed,
		"disp_textcolor": rgd.cfg.disp_textcolor,
		"disp_backgroundcolor": rgd.cfg.disp_backgroundcolor,
		"disp_orientation": rgd.cfg.disp_orientation,
		"disp_hmargin": rgd.cfg.disp_hmargin,
		"disp_vmargin": rgd.cfg.disp_vmargin,
		"disp_fontsize": rgd.cfg.disp_fontsize,
		"disp_fullscreen": rgd.cfg.disp_fullscreen,
	});
	Ok(body)
}

pub async fn lapi_config_set(d: Arc<Mutex<LpfHttpServerData>>, params: Value) -> Result<Value, HTTPAPIError>
{
	let ud = d.lock().await;
	let mut rgd = ud.runtime_global_data.lock().await;

	let cfgval: &Value = match params.get("cfg") {
		Some(v) => v,
		None => {
			let r = json!({ "err": "Unable to find cfg in request parameters" }).to_string();
			return Err(HTTPAPIError::Message{description: r.to_string()})
		}
	};

	rgd.cfg.set_partial_cfg(cfgval);

	rgd.cfg.save().await ?;


	let body = json!({
		"risultato": 0
	});
	Ok(body)
}
