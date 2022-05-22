document.addEventListener("DOMContentLoaded", function(event) {
	readConfig();

	document.getElementById('btnApply').addEventListener('click', (e) => {
		do_click_apply();
	});

	document.querySelector('#bgfile input[type=file]').addEventListener('change', (e) => {
		const fileInput = document.querySelector('#bgfile input[type=file]');
		const fileName = document.querySelector('#bgfile .file-name');
		fileName.textContent = fileInput.files[0].name;
	});

	bulmaNavbarEnable();
	bulmaNotifEnable();

});

async function do_click_apply()
{
		const fileInput = document.querySelector('#bgfile input[type=file]');
		showNotification(null);

		try {
			if (fileInput.files.length > 0)
				await uploadBackground();
			await saveConfig();
			readConfig();
		} catch(msg) {
			showNotification(msg);
		}
}

async function uploadBackground()
{
	const fileInput = document.querySelector('#bgfile input[type=file]');
	let formData = new FormData();
	formData.append("file", fileInput.files[0]);
	let rsp = await fetch('/upload', {
		method: 'POST',
		body: formData
	});
	console.log(rsp);
	console.log('file uploaded');
}

function showNotification(msg) {
	let n = document.getElementById('notification1');
	let nt = n.querySelector('.notiftext');
	if (msg == null) {
		nt.innerText = '';
		n.classList.add('is-hidden');
	} else {
		nt.innerText = msg;
		n.classList.remove('is-hidden');
	}
}

function bulmaNotifEnable() {
	document.querySelectorAll('.notification .delete').forEach( del => {
		del.addEventListener('click', () => {
			del.parentNode.classList.add('is-hidden');
		});
	});
}

function bulmaNavbarEnable() {
		const $navbarBurgers = Array.prototype.slice.call(document.querySelectorAll('.navbar-burger'), 0);
		if ($navbarBurgers.length > 0) {
			$navbarBurgers.forEach( el => {
				el.addEventListener('click', () => {
					const target = el.dataset.target;
					const $target = document.getElementById(target);
					el.classList.toggle('is-active');
					$target.classList.toggle('is-active');
				});
			});
		}
}


function fillform(cfg) {
	document.getElementById('currenttext').value = cfg.disp_text;
	document.getElementById('newtext').value = cfg.disp_text;
	document.getElementById('scrollspeed').value = cfg.disp_scrollspeed;
	document.getElementById('backgroundcolor').value = cfg.disp_backgroundcolor;
	document.getElementById('textcolor').value = cfg.disp_textcolor;
	document.getElementById('hmargin').value = cfg.disp_hmargin;
	document.getElementById('vmargin').value = cfg.disp_vmargin;
	document.getElementById('fontsize').value = cfg.disp_fontsize;

	document.querySelectorAll('input[name="orientation"]').forEach((input) => input.checked = false);
	document.querySelector('input[name="orientation"][value="' + cfg.disp_orientation+ '"]').checked = true;

	document.querySelectorAll('input[name="fullscreen"]').forEach((input) => input.checked = false);
	document.querySelector('input[name="fullscreen"][value="' + cfg.disp_fullscreen+ '"]').checked = true;

}


function readConfig()
{
	fetch('/lapi', {
		method: 'POST',
		body: JSON.stringify({cmd: 'config_get'})
	})
	.then(response => response.json())
	.then(d => {
		if (d.auth != undefined) {
			document.location = "auth.html";
			return;
		}
		if (d.err != undefined) {
			console.log(d.err);
			return;
		}
		fillform(d);
	})
	.catch(err => {
		console.log(err);
	});
}

function getNumberFromForm(id, defaultval)
{
	let v = parseInt(document.getElementById(id).value);
	if (isNaN(v))
		return defaultval;
	else
		return v;
}

function enableControls(enable)
{
	if (!enable) {
		document.getElementById('btnApply').classList.add('is-loading');
		document.querySelectorAll('input').forEach((e) => {e.disabled = true;});
		document.querySelectorAll('textarea').forEach((e) => {e.disabled = true;});
		document.querySelectorAll('button').forEach((e) => {e.disabled = true;});
	} else {
		document.getElementById('btnApply').classList.remove('is-loading');
		document.querySelectorAll('input').forEach((e) => {e.disabled = false;});
		document.querySelectorAll('textarea').forEach((e) => {e.disabled = false;});
		document.querySelectorAll('button').forEach((e) => {e.disabled = false;});
	}
}

function saveConfig()
{
	return new Promise((resolve, reject) => {
		enableControls(false);
		let cfg = {
			disp_text: document.getElementById('newtext').value,
			disp_scrollspeed: getNumberFromForm('scrollspeed', 1),
			disp_hmargin: getNumberFromForm('hmargin', 10),
			disp_vmargin: getNumberFromForm('vmargin', 10),
			disp_fontsize: getNumberFromForm('fontsize', 24),
			disp_backgroundcolor: document.getElementById('backgroundcolor').value,
			disp_textcolor: document.getElementById('textcolor').value,
			disp_orientation: document.querySelector('input[name="orientation"]:checked').value,
			disp_fullscreen: document.querySelector('input[name="fullscreen"]:checked').value == "true"
		};
		fetch('/lapi', {
			method: 'POST',
			body: JSON.stringify({cmd: 'config_set', cfg: cfg })
		})
		.then(response => response.json())
		.then(d => {
			if (d.auth != undefined) {
				document.location = "auth.html";
				reject('auth error');
				return;
			}
			if (d.err != undefined) {
				let merr = 'Server error while saving config: ' + d.err;
				enableControls(true);
				reject(merr);
				return;
			}
			enableControls(true);
			resolve();
		})
		.catch(err => {
			let merr = 'Error while saving config: ' + err;
			enableControls(true);
			reject(merr);
		});
	});
}

