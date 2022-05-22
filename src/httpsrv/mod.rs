use std::convert::Infallible;


use hyper::{Body, Request, Response, Server, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use hyper::header::HeaderValue;
use hyper::server::conn::AddrStream;

use pwhash::bcrypt;

use std::str;

use tokio::fs;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Instant};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::ffi::OsStr;
use std::fmt;

use cookie::Cookie;
use chrono::{Utc};

use crate::lconfig::{RuntimeGlobalData};

const COOKIE_HEADER_NAME : &str = "Cookie";
const _USER_AGENT_HEADER_NAME : &str = "User-Agent";
const AUTH_COOKIE_NAME : &str = "TXTSCROLLSESSID";
const AUTHENTICATION_PAGE : &str = "/auth.html";
const SESSION_TIMEOUT: u64 = 1800;

mod api_config;
use api_config::lapi_config_get;
use api_config::lapi_config_set;

mod api_pwd;
use api_pwd::lapi_password_change;

use crate::{APP_NAME_APPLICATION, APP_VERSION};


#[derive(Serialize, Deserialize)]
struct AuthParams {
	username: String,
	password: String
}

#[allow(dead_code)]
#[derive(Clone)]
struct BrowserSession {
	username: String,
	start_time: Instant,
	last_seen: Instant,
	remote_addr: SocketAddr,
}

#[derive(Clone)]
pub struct LpfHttpServerData {
	server_start_time: Instant,
	authenticated_sessions: HashMap<String, BrowserSession>,
	runtime_global_data: Arc<Mutex<RuntimeGlobalData>>
}

#[derive(Debug)]
pub enum HTTPAPIError {
    Message { description: String },
    IoError(std::io::Error),
}

impl From<std::io::Error> for HTTPAPIError {
    fn from(err: std::io::Error) -> Self {
        HTTPAPIError::IoError(err)
    }
}

impl fmt::Display for HTTPAPIError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // write!(f, "{}", self.details)
        match &*self {
			HTTPAPIError::Message { description } => write!(f, "{}", description),
            HTTPAPIError::IoError(ref e) => e.fmt(f),
        }
    }
}
fn get_session_sid(req: &Request<Body>) -> Option<String> {
    /* Get the Cookie: header as a string */
    let ch = match req.headers().get(COOKIE_HEADER_NAME) {
        Some(v) => v,
        None => return None
    };
    let s = match ch.to_str() {
        Ok(v) => v,
        Err(_e) => return None
    };
    /* Split s into cookies */
    let isplit = s.split(";");
    /* Search our session id in AUTH_COOKIE_NAME */
    let mut sid = String::from("");
    for sc in isplit {
        // println!("   {}",sc);
        if let Ok(ck) = Cookie::parse(sc) {
            if ck.name() == AUTH_COOKIE_NAME {
                sid = ck.value().to_owned();
            }
        }
    }

	if !sid.starts_with("S-") {
		return None;
	}

	/* Remove leading S- from sid */
	let rsid = sid.drain(2..).collect();

    Some(rsid)

}

async fn is_authenticated(d: Arc<Mutex<LpfHttpServerData>>, req: &Request<Body>) -> Option<String> {

	let sid = match get_session_sid(req) {
		Some(sid) => sid,
		None => return None
	};

   	/* Lock global configuration data */
	let mut rd = d.lock().await;

	/* Check if session exists and is not expired */
	match rd.authenticated_sessions.get_mut(&sid) {
		Some(ses) => {
			let dur_sec = ses.last_seen.elapsed().as_secs();
			if dur_sec > SESSION_TIMEOUT {
				println!("Session {} expired", sid);
				rd.authenticated_sessions.remove(&sid);
				None
			} else {
				// update last_seen
				ses.last_seen = Instant::now();
				Some(sid)
			}
		},
		None => {
			println!("Session {} not found in authenticated_session table", sid);
			None
		}
	}

}

async fn collect_json_post_request(req: Request<Body>) -> Result<Value, String> {
	// to do: versione generalizzata di collect_auth_params
	let body = match hyper::body::to_bytes(req.into_body()).await {
		Ok(b) => b,
		Err(e) => return Err(e.to_string())
	};

	let v: Value = match serde_json::from_slice(body.as_ref()) {
		Ok(v) => v,
		Err(e) => return Err(format!("Unable to parse json POST body: {}", e.to_string()))
	};

	Ok(v)

}

async fn collect_auth_params(req: Request<Body>) -> Result<AuthParams, String> {
	let body = match hyper::body::to_bytes(req.into_body()).await {
		Ok(b) => b,
		Err(e) => return Err(e.to_string())
	};
	let j : AuthParams = match serde_json::from_slice(body.as_ref()) {
		Ok(v) => v,
		Err(e) => return Err(format!("Unable to parse json POST body: {}", e.to_string()))
	};

	// println!("Auth requested: {} {}", j.username, j.password);

	Ok(j)
}

async fn validate_credentials(d: &Arc<Mutex<LpfHttpServerData>>, ap: &AuthParams) -> bool
{

	let ud = d.lock().await;
	let rgd = ud.runtime_global_data.lock().await;
	let encrypted_admin_password = rgd.cfg.encrypted_admin_password.clone();
	drop(rgd);

	if ap.username != "admin" {
		return false;
	}

	if !bcrypt::verify(ap.password.clone(), &encrypted_admin_password) {
		return false;
	}

	true

}

fn content_type(filename: PathBuf) -> &'static str
{

	match filename.extension().and_then(OsStr::to_str) {
		Some("png") => "image/png",
		Some("html") => "text/html",
		Some("js") => "text/javascript",
		Some("jpg") => "image/jpeg",
		Some("txt") => "text/plain",
		Some("css") => "text/css",
		_ => "application/octet-stream"
	}

}


async fn expire_auth_sessions(d: Arc<Mutex<LpfHttpServerData>>) {
	let mut rd = d.lock().await;
	rd.authenticated_sessions.retain(|_key, bs| {
		let age_sec = bs.last_seen.elapsed().as_secs();
		if age_sec > SESSION_TIMEOUT {
// 			println!("Expiring old session {},{} IP:{:?}", key, bs.last_seen.elapsed().as_secs(), bs.remote_addr);
		}
		!(age_sec > SESSION_TIMEOUT)
	});
}

fn generate_new_sid(start_ref_time: std::time::Instant) -> String {
	/* Generate a session ID just from current time.  To be improved. */
	let sid = start_ref_time.elapsed().as_secs();
	println!("New SID {} generated", sid);
	format!("{}", sid)
}

fn append_session_cookie_to_response(response: &mut Response<Body>, sid: String) {
	let cookieval = format!("S-{}", sid);
	/* let cookie_header_str = format!("{}={}; SameSite=Strict; Max-Age={}", AUTH_COOKIE_NAME, cookieval, COOKIE_MAX_AGE); */
	let cookie_header_str = format!("{}={}; SameSite=Strict", AUTH_COOKIE_NAME, cookieval);
	let cookie_header = HeaderValue::from_str(&cookie_header_str).unwrap();
	response.headers_mut().insert("Set-Cookie", HeaderValue::from(cookie_header));
}


async fn serve_authservice(d: Arc<Mutex<LpfHttpServerData>>, req: Request<Body>, response: &mut Response<Body>, client_addr: SocketAddr) {
	response.headers_mut().insert("Content-type", HeaderValue::from_static("application/json"));
	let ap = match collect_auth_params(req).await {
		Ok(a) => a,
		Err(e) => {
			let r = json!({ "err": e }).to_string();
			*response.body_mut() = Body::from(r);
			return;
		}
	};

	expire_auth_sessions(d.clone()).await;


	if validate_credentials(&d, &ap).await {
		let mut rd = d.lock().await;
		let sid = generate_new_sid(rd.server_start_time);
		append_session_cookie_to_response(response, sid.clone());
		*response.body_mut() = Body::from(r#"{ "auth": "ok"}"#);
		let ses = BrowserSession {
			username: ap.username,
			start_time: Instant::now(),
			last_seen: Instant::now(),
			remote_addr: client_addr
		};
		rd.authenticated_sessions.insert(sid.clone(), ses);
		println!("Authenticated new session with sid = {}", sid);
	} else {
		let r = json!({ "err": "Invalid username or password"}).to_string();
		*response.body_mut() = Body::from(r);
	}
}

async fn serve_logoff(d: Arc<Mutex<LpfHttpServerData>>, req: Request<Body>, response: &mut Response<Body>, _client_addr: SocketAddr)
{
	let sid = match get_session_sid(&req) {
		Some(sid) => sid,
		None => return ()
	};

   	/* Lock global configuration data */
    let mut rd = d.lock().await;

	/* Check if session exists and is not expired */
	match rd.authenticated_sessions.get(&sid) {
		Some(_ses) => {
			println!("Logging off session {}", sid);
			rd.authenticated_sessions.remove(&sid);
		},
		None => ()
	}

	*response.body_mut() = Body::from(format!("Redirecting to {}", AUTHENTICATION_PAGE));
	response.headers_mut().insert("Location", HeaderValue::from_static(AUTHENTICATION_PAGE));
	*response.status_mut() = StatusCode::FOUND;

}

async fn serve_upload(d: Arc<Mutex<LpfHttpServerData>>, req: Request<Body>, response: &mut Response<Body>, _client_addr: SocketAddr) {
	response.headers_mut().insert("Content-type", HeaderValue::from_static("application/json"));

	let _sid = match is_authenticated(d.clone(), &req).await {
		Some(s) => s,
		None => {
			let r = json!({ "auth": "not authenticated or session expired" }).to_string();
			*response.body_mut() = Body::from(r);
			return
		}
	};

	let errj = json!({"err": "Upload error"});
	*response.body_mut() = Body::from(errj.to_string());
}

async fn serve_lapi(d: Arc<Mutex<LpfHttpServerData>>, req: Request<Body>, response: &mut Response<Body>, _client_addr: SocketAddr) {
	response.headers_mut().insert("Content-type", HeaderValue::from_static("application/json"));

	let _sid = match is_authenticated(d.clone(), &req).await {
		Some(s) => s,
		None => {
			let r = json!({ "auth": "not authenticated or session expired" }).to_string();
			*response.body_mut() = Body::from(r);
			return
		}
	};

	// also resend cookie with new expire time
	/* append_session_cookie_to_response(response, sid); */

	let v = match collect_json_post_request(req).await {
		Ok(v) => v,
		Err(e) => {
			let r = json!({ "err": e }).to_string();
			*response.body_mut() = Body::from(r);
			return;
		}
	};

	let cmdval: &Value = match v.get("cmd") {
		Some(v) => v,
		None => {
			let r = json!({ "err": "unable to find cmd in json data" }).to_string();
			*response.body_mut() = Body::from(r);
			return;
		}
	};

	let cmd: &str = match cmdval.as_str() {
		Some(c) => c,
		None => {
			let r = json!({ "err": "cmd is not a string" }).to_string();
			*response.body_mut() = Body::from(r);
			return;
		}
	};

	let lapi_result = match cmd {
		"config_get" => lapi_config_get(d, v).await,
		"config_set" => lapi_config_set(d, v).await,
		"password_change" => lapi_password_change(d, v).await,
		_ => {
			let r = json!({ "err": format!("{} is not recognized as a lapi cmd", cmd) }).to_string();
			*response.body_mut() = Body::from(r);
			return;
		}
	};


	match lapi_result {
		Ok(jresult) => *response.body_mut() = Body::from(jresult.to_string()),
		Err(herr) => {
			let errj = json!({"err": herr.to_string() });
			*response.body_mut() = Body::from(errj.to_string());
		}
	}
}

fn strip_heading_slashes(u: &str) -> String {
	let mut idx = 0;
	let ubytes = u.as_bytes();
	static SLASH: u8 = '/' as u8;
	while idx < ubytes.len() && ubytes[idx] == SLASH {
		idx += 1;
	}
	u[idx..].to_owned()
}


async fn serve_page(d: Arc<Mutex<LpfHttpServerData>>, req: Request<Body>, response: &mut Response<Body>, client_addr: SocketAddr) {
		let now = Utc::now();
		println!("{} {} URI: {} Method: {}",
				now.to_rfc3339(),
				client_addr.ip(), req.uri().path(), req.method());
		/* Missing authentication redirect is required for everything *.html  */
		let p = req.uri().path();
		let dc1 = d.clone();
		if (p == "/" || p.to_lowercase().ends_with(".html")) &&
				is_authenticated(dc1, &req).await == None &&
				p != AUTHENTICATION_PAGE
		{
				*response.body_mut() = Body::from(format!("While serving static page, authentication is needed. Redirecting to {}", AUTHENTICATION_PAGE));
				response.headers_mut().insert("Location", HeaderValue::from_static(AUTHENTICATION_PAGE));
				*response.status_mut() = StatusCode::FOUND;
		} else {
				if req.uri() == "/authservice" && req.method() == &Method::POST {
						serve_authservice(d, req, response, client_addr).await;
				} else if req.uri() == "/lapi" && req.method() == &Method::POST {
						serve_lapi(d, req, response, client_addr).await;
				} else if req.uri() == "/upload" && req.method() == &Method::POST {
						serve_upload(d, req, response, client_addr).await;
				} else if req.uri() == "/logoff.do" && req.method() == &Method::GET {
						serve_logoff(d, req, response, client_addr).await;
				} else {
						/* Serves a static file.
							/ is translated to /index.html
							and /fonts is translated into runtime_data_dir+"/fonts"
							and /media is translated into runtime_data_dir+"/media"
						*/
						let ld = d.lock().await;
						let rgd = ld.runtime_global_data.lock().await;
						let mut filename = rgd.html_dir.clone();
						let slash_stripped_uri = strip_heading_slashes(req.uri().path());
						if req.uri() == "/" {
								filename.push("index.html");
						} else {
								filename.push(slash_stripped_uri);
						}
						drop(rgd);

						let (mut content, content_type) = match fs::read(&filename).await {
								Ok(s) => (s, content_type(filename)),
								Err(e) => {
									// File not found or other similar error
									*response.status_mut() = StatusCode::NOT_FOUND;
									println!("Unalbe to read file {}: {}", filename.display(), e.to_string());
									(format!("Unalbe to read file {}: {}", filename.display(), e.to_string()).into_bytes(), "text/plain")
								}
						};

						/* Change some variables when showing html, like {appname} */
						if content_type == "text/html" {
							let mut cs = match str::from_utf8(&content) {
								Ok(v) => v.to_string(),
								Err(_e) => String::from("Unable to convert html file to UTF8 string. Invalid UTF-8 chars?")
							};
							cs = cs
								.replace("{appname}", APP_NAME_APPLICATION)
								.replace("{appversion}", APP_VERSION);
							content = Vec::from(cs);
						}

						response.headers_mut().insert("Content-type", HeaderValue::from_static(content_type));
						*response.body_mut() = Body::from(content);
				}
	}
}

async fn servicefn(d: Arc<Mutex<LpfHttpServerData>>, req: Request<Body>, client_addr: SocketAddr) -> Response<Body> {
    let mut response = Response::new(Body::empty());
	let method = req.method();


    if method == &Method::GET || method == &Method::POST {
        // serve_page(d, req, &mut response, client_addr).await;
        serve_page(d, req, &mut response, client_addr).await;
    } else {
        *response.body_mut() = Body::from("Page not found or invalid method");
        *response.status_mut() = StatusCode::NOT_FOUND;
    }
    response
}

pub async fn httpd_main(runtime_global_data: &Arc<Mutex<RuntimeGlobalData>>) -> Result<(), std::io::Error> {
	let cf = runtime_global_data.lock().await;

	let d = Arc::new(Mutex::new(
			LpfHttpServerData {
					authenticated_sessions: HashMap::new(),
					server_start_time: Instant::now(),
					runtime_global_data: runtime_global_data.clone()
	}));

	let http_port = cf.cfg.http_port;
	drop(cf);

	let addr = ([0,0,0,0], http_port).into();

	let make_svc = make_service_fn(move |conn: &AddrStream| {
		let addr = conn.remote_addr();
		let d = d.clone();
		async move {
			Ok::<_, Infallible>(service_fn(move |req: hyper::Request<Body>| {
				let addr = addr.clone();
				let d = d.clone();
				async move {
					Ok::<_,Infallible>(servicefn(d, req, addr.clone()).await)
				}
			}))
		}

	});


	let server = Server::bind(&addr).serve(make_svc);
	println!("HTTP server listening on port {}", http_port);

	// Run this server for... forever!
	if let Err(e) = server.await {
		eprintln!("server error: {}", e);
	}
	println!("Gio#2");
	Ok(())

}
