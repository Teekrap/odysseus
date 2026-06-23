import { invoke } from '@tauri-apps/api/core';

const urlInput = document.getElementById('url');
const connectBtn = document.getElementById('connect');
const status = document.getElementById('status');

async function loadStored() {
  try {
    const stored = await invoke('get_stored_url');
    if (typeof stored === 'string' && stored) {
      urlInput.value = stored;
    }
  } catch {}
}

async function connect() {
  const url = urlInput.value.trim();
  if (!url) {
    status.textContent = 'Enter a server URL.';
    status.className = 'status err';
    return;
  }
  if (!/^https?:\/\//i.test(url)) {
    status.textContent = 'URL must start with http:// or https://';
    status.className = 'status err';
    return;
  }
  connectBtn.disabled = true;
  status.textContent = 'Probing server...';
  status.className = 'status';
  try {
    const ok = await invoke('connect', { url });
    if (ok) {
      status.textContent = 'Connected. Loading Odysseus...';
      status.className = 'status ok';
    } else {
      status.textContent = 'No response at that URL. Check the address and that Odysseus is running.';
      status.className = 'status err';
      connectBtn.disabled = false;
    }
  } catch (e) {
    status.textContent = String(e);
    status.className = 'status err';
    connectBtn.disabled = false;
  }
}

connectBtn.addEventListener('click', connect);
urlInput.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') connect();
});

loadStored().then(() => urlInput.focus());
