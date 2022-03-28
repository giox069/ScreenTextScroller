
use speedy2d::Window;
use speedy2d::color::Color;
use speedy2d::window::{WindowHandler, WindowHelper, WindowStartupInfo, WindowFullscreenMode, MouseButton};
use speedy2d::Graphics2D;
use speedy2d::font::Font;
use speedy2d::font::TextLayout;
use speedy2d::font::TextOptions;
use speedy2d::font::TextAlignment;
use speedy2d::font::FormattedTextBlock;
use speedy2d::dimen::Vector2;
use speedy2d::shape::Rectangle;


use std::rc::Rc;
use tokio::task;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::block_in_place;
use std::path::PathBuf;

use image::io::Reader;
use image::error::ImageError;


use lconfig::{RuntimeGlobalData, TextScrollOrientation};
use lconfig::Config;

use std::time::Instant;

mod lconfig;
mod httpsrv;

const APP_NAME_APPLICATION: &str = env!("CARGO_PKG_NAME");
const APP_NAME_ORGANIZATION: &str = "giox069";
const APP_NAME_QUALIFIER: &str = "com";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

const FULLSCREEN_MOUSE_HIDE_MILLISEC: u128 = 3000;

struct MyWindowHandler {
    font: Font,
    y: f32,
	x: f32,
    size: Vector2<u32>,
    rgd: Arc<Mutex<RuntimeGlobalData>>,
    current_cfg_copy: Config,
    pause: bool,
    block: Option<Rc<FormattedTextBlock>>,
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    last_mouse_move: Instant,
    mouse_hidden: bool
}


// https://docs.rs/speedy2d/latest/speedy2d/

impl MyWindowHandler {
	fn rebuild_text_block(&mut self) {
		
		let mut text_options = TextOptions::new();
		if self.current_cfg_copy.disp_orientation == TextScrollOrientation::ScrollVertical {
			text_options = text_options.with_wrap_to_width(
				(self.size.x - 2 * self.current_cfg_copy.disp_hmargin as u32) as f32,
				TextAlignment::Left);
		};

		let block = self.font.layout_text(&self.current_cfg_copy.disp_text,
			self.current_cfg_copy.disp_fontsize as f32,
			text_options);

		self.max_y = self.size.y as f32 - self.current_cfg_copy.disp_vmargin as f32;
		self.min_y = self.current_cfg_copy.disp_vmargin as f32 - block.height();
		self.max_x = self.size.x as f32 - self.current_cfg_copy.disp_hmargin as f32;
		self.min_x = self.current_cfg_copy.disp_hmargin as f32 - block.width();

		self.block = Some(block);


	}
	fn new(rgd: Arc<Mutex<RuntimeGlobalData>>) -> MyWindowHandler {
		let bytes = include_bytes!("../assets/fonts/Ubuntu-R.ttf");
		let font = Font::new(bytes).unwrap();

		MyWindowHandler {
			font: font,
			y: 0.0,
			x: 0.0,
			size: Vector2{x: 10, y:10},
			rgd: rgd,
			current_cfg_copy: Config::new(),
			pause: false,
			block: None,
			min_x: 0.0,
			max_x: 100.0,
			min_y: 0.0,
			max_y: 100.0,
			last_mouse_move: Instant::now(),
			mouse_hidden: false
			}
	}

	fn load_icon(&self, helper: &mut WindowHelper) -> std::result::Result<(), ImageError> {

		/* Load the icon */
		let rgd = self.rgd.blocking_lock();
		let mut icon_filename = rgd.runtime_data_dir.clone();
		drop(rgd);
		icon_filename.push("icons");
		icon_filename.push("main_icon.png");

		let image = Reader::open(icon_filename)?.decode()?;

		let s = (image.width(), image.height());
		let imga32 = image.into_rgba8();


		match helper.set_icon_from_rgba_pixels(imga32.into_vec(), s) {
			Ok(_) => (),
			Err(e) => println!("Unable to set window icon: {}", e)
		};


		println!("Icon image loaded. Size is {}x{}.", s.0, s.1);
		Ok(())
	}
}

impl WindowHandler for MyWindowHandler
{


	fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D) {
		// println!("On draw");

		let rgd = self.rgd.blocking_lock();
		let mut config_has_changed = false;
		if self.current_cfg_copy.version != rgd.cfg.version {
			self.current_cfg_copy = rgd.cfg.clone();
			config_has_changed = true;
		}
		drop(rgd);

		if config_has_changed {
			self.y = match self.current_cfg_copy.disp_orientation {
				TextScrollOrientation::ScrollVertical => {
					if self.current_cfg_copy.disp_scrollspeed < 0 {
						self.max_y
					} else {
						self.min_y
					}
				},
				TextScrollOrientation::ScrollHorizontal => self.current_cfg_copy.disp_vmargin as f32
			};
			self.x = match self.current_cfg_copy.disp_orientation {
				TextScrollOrientation::ScrollHorizontal => {
					if self.current_cfg_copy.disp_scrollspeed < 0 {
						self.max_x
					} else {
						self.min_x
					}
				},
				TextScrollOrientation::ScrollVertical => self.current_cfg_copy.disp_hmargin as f32
			};


			self.rebuild_text_block();
			if self.current_cfg_copy.disp_fullscreen {
				// Enter fullscreen mode
				helper.set_fullscreen_mode(WindowFullscreenMode::FullscreenBorderless);
				if !self.mouse_hidden {
					self.last_mouse_move = Instant::now();
				}
			} else {
				// Exit fullscreen mode
				if self.mouse_hidden {
					self.mouse_hidden = false;
					helper.set_cursor_visible(true);
				}
				helper.set_fullscreen_mode(WindowFullscreenMode::Windowed);
			}
		}

		let bgcolor = match csscolorparser::parse(&self.current_cfg_copy.disp_backgroundcolor) {
			Ok(c) => Color::from_rgb(c.r as f32, c.g as f32, c.b as f32),
			Err(_) => Color::BLACK
		};

		let clip_area = Rectangle::from_tuples(
			(self.current_cfg_copy.disp_hmargin as i32, self.current_cfg_copy.disp_vmargin as i32 ),
			(self.size.x as i32 - self.current_cfg_copy.disp_hmargin as i32, self.size.y as i32 - self.current_cfg_copy.disp_vmargin as i32)
		);

		graphics.clear_screen(bgcolor);

		match &self.block {
			Some(b) => {

				let fgcolor = match csscolorparser::parse(&self.current_cfg_copy.disp_textcolor) {
					Ok(c) => Color::from_rgb(c.r as f32, c.g as f32, c.b as f32),
					Err(_) => Color::WHITE
				};

				graphics.set_clip(Some(clip_area));

				graphics.draw_text((
						self.x,
						self.y
					),
					fgcolor, &b);

				// println!("self.size.x={} self.size.y={} self.x={} self.y={}", self.size.x, self.size.y, self.x, self.y);

				if !self.pause {
					match self.current_cfg_copy.disp_orientation {
						TextScrollOrientation::ScrollVertical => {
							self.y -= self.current_cfg_copy.disp_scrollspeed as f32;

							if self.y >=  self.max_y {
								self.y = self.min_y;
							}
							if self.y < self.min_y {
								self.y = self.max_y;
							}
						},
						TextScrollOrientation::ScrollHorizontal => {
							self.x -= self.current_cfg_copy.disp_scrollspeed as f32;

							if self.x >=  self.max_x {
								self.x = self.min_x;
							}
							if self.x < self.min_x {
								self.x = self.max_x;
							}
						}
					}
				}

			},
			None => {}
		}

		if self.current_cfg_copy.disp_fullscreen && !self.mouse_hidden {
			if self.last_mouse_move.elapsed().as_millis() > FULLSCREEN_MOUSE_HIDE_MILLISEC {
				self.mouse_hidden = true;
				helper.set_cursor_visible(false);
			}
		}

		// Request that we draw another frame once this one has finished
		helper.request_redraw();
	}

	fn on_resize(&mut self, _helper: &mut WindowHelper, size_pixels: Vector2<u32>) {

		self.size = size_pixels;
		self.rebuild_text_block();
	}

	fn on_start(&mut self, helper: &mut WindowHelper, info: WindowStartupInfo) {

		helper.set_title(APP_NAME_APPLICATION);

		match self.load_icon(helper) {
			Ok(_) => (),
			Err(e) => println!("Icon image load error: {}", e)
		};

		self.size = info.viewport_size_pixels().clone();
		self.rebuild_text_block();
	}

	fn on_mouse_button_down(&mut self, _helper: &mut WindowHelper<()>, button: MouseButton) {
		if button == MouseButton::Right {
			self.pause = !self.pause;
		}
	}

	fn on_mouse_move(&mut self, helper: &mut WindowHelper<()>, _position: Vector2<f32>) {
		self.last_mouse_move = Instant::now();
		if self.mouse_hidden {
			self.mouse_hidden = false;
			helper.set_cursor_visible(true);
		}
	}

	fn on_scale_factor_changed(&mut self, _helper: &mut WindowHelper<()>, scale_factor: f64) {
		println!("Scale factor changed: {}", scale_factor);
	}

}

#[tokio::main]
async fn main() {

	/* Try to find where our runtime-data directory is and set runtime_data_dir */

	let runtime_data_dir_relpath = PathBuf::from("./runtime-data");
	let mut runtime_data_dir = match tokio::fs::canonicalize(runtime_data_dir_relpath.clone()).await {
		Ok(p) => p,
		Err(_e) => runtime_data_dir_relpath
	};
	if !runtime_data_dir.exists() {
		runtime_data_dir = PathBuf::from("/usr/share/").join(APP_NAME_APPLICATION);
	}


	let html_dir = runtime_data_dir.join("html");
	let cfg : lconfig::Config = lconfig::Config::load().await;


	let runtime_global_data = Arc::new(Mutex::new(
		RuntimeGlobalData {
				runtime_data_dir: runtime_data_dir,
				html_dir: html_dir,
				cfg: cfg
		}
	));

	let rgdclone = runtime_global_data.clone();

	task::spawn(async move {httpsrv::httpd_main(&runtime_global_data).await});

	let window = Window::new_centered("Title",(640, 480)).unwrap();

	let wh = MyWindowHandler::new(rgdclone);
	println!("Starting window loop");
	block_in_place(move || {window.run_loop(wh)});

}
