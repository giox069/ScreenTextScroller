document.addEventListener("DOMContentLoaded", function(event) {

  document.getElementById('btnChange').addEventListener('click', (e) => {
	  showNotification(null);

	  let npass1 = document.getElementById('frmNewPassowrd1').value;
	  let npass2 = document.getElementById('frmNewPassowrd2').value;
	  if (npass1 != npass2) {
		  showNotification('New passwords does not match');
		  return;
	  }

	  let oldpass = document.getElementById('frmOldPassword').value;

	  changePassword(oldpass, npass1)
	  .then(() => {
		showNotification('passowrd changed');
		document.location = "chadmpwdok.html";
		})
	  .catch(msg => showNotification(msg));
  });

	bulmaNotifEnable();

});

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


function enableControls(enable)
{
	if (!enable) {
		document.getElementById('btnChange').classList.add('is-loading');
		document.querySelectorAll('input').forEach((e) => {e.disabled = true;});
		document.querySelectorAll('textarea').forEach((e) => {e.disabled = true;});
		document.querySelectorAll('button').forEach((e) => {e.disabled = true;});
	} else {
		document.getElementById('btnChange').classList.remove('is-loading');
		document.querySelectorAll('input').forEach((e) => {e.disabled = false;});
		document.querySelectorAll('textarea').forEach((e) => {e.disabled = false;});
		document.querySelectorAll('button').forEach((e) => {e.disabled = false;});
	}
}

function changePassword(oldpass, newpass)
{
	return new Promise((resolve, reject) => {
		enableControls(false);
		let rq = {
			cmd: 'password_change',
			oldpass: oldpass,
			newpass: newpass
		};
		fetch('/lapi', {
			method: 'POST',
			body: JSON.stringify(rq)
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
			if (d.rc == 1) {
				let merr = 'Invalid old password, please retry.';
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

