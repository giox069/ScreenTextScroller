
use tokio::sync::Mutex;
use std::sync::Arc;
use serde_json::json;
use serde_json::Value;
use pwhash::bcrypt;

use crate::httpsrv::LpfHttpServerData;
use crate::httpsrv::HTTPAPIError;


pub async fn lapi_password_change(d: Arc<Mutex<LpfHttpServerData>>, params: Value) -> Result<Value, HTTPAPIError>
{
	let ud = d.lock().await;
	let mut rgd = ud.runtime_global_data.lock().await;

	let oldpassv: &Value = match params.get("oldpass") {
		Some(v) => v,
		None => {
			let r = json!({ "err": "Unable to find oldpass in request parameters" });
			return Err(HTTPAPIError::Message{description: r.to_string()})
		}
	};

	let newpassv: &Value = match params.get("newpass") {
		Some(v) => v,
		None => {
			let r = json!({ "err": "Unable to find newpass in request parameters" });
			return Err(HTTPAPIError::Message{description: r.to_string()})
		}
	};

	let oldpass= match oldpassv.as_str() {
		Some(s) => s,
		None => ""
	};


	let newpass= match newpassv.as_str() {
		Some(s) => s,
		None => ""
	};

	if !bcrypt::verify(oldpass, &rgd.cfg.encrypted_admin_password) {
		let r = json!({ "rc": 1, "description": "Old password is invalid"});
		return Ok(r);
	}

	let encrypted_password = match bcrypt::hash(newpass) {
		Ok(s) => s,
		Err(_e) => String::from("")
	};


	println!("Encrypted new password is {}", encrypted_password);

	rgd.cfg.encrypted_admin_password = encrypted_password;

	rgd.cfg.save().await ?;

	let body = json!({
		"rc": 0,
		"description": "Password changed"
	});
	Ok(body)
}
