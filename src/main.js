const { listen } = window.__TAURI__.event;
const { invoke } = window.__TAURI__.core;

let currentSettings = null;
let showSettings = false;

function formatBytes(bytes) {
  const mb = bytes / (1024 * 1024);
  if (mb >= 1024) {
    return `${(mb / 1024).toFixed(1)} GB`;
  }
  return `${Math.round(mb)} MB`;
}

function tempColor(temp) {
  if (temp < 50) return 'bar-green';
  if (temp < 75) return 'bar-yellow';
  return 'bar-red';
}

function utilizationColor(pct) {
  if (pct < 50) return 'bar-green';
  if (pct < 85) return 'bar-yellow';
  return 'bar-red';
}

function convertTemp(celsius, unit) {
  if (unit === 'f') return Math.round(celsius * 9 / 5 + 32);
  return celsius;
}

function tempUnit() {
  return currentSettings?.temp_unit === 'f' ? 'F' : 'C';
}

function renderGpuCard(gpu) {
  const memPct = gpu.memory_total > 0 ? (gpu.memory_used / gpu.memory_total * 100).toFixed(0) : 0;
  const powerPct = gpu.power_limit > 0 ? (gpu.power_usage / gpu.power_limit * 100).toFixed(0) : 0;
  const temp = convertTemp(gpu.temperature, currentSettings?.temp_unit);

  let processesHtml = '';
  if (gpu.processes.length > 0) {
    const rows = gpu.processes.map(p =>
      `<div class="process-row">
        <span class="process-name">${escapeHtml(p.name)} (${p.pid})</span>
        <span class="process-mem">${formatBytes(p.memory_used)}</span>
      </div>`
    ).join('');
    processesHtml = `
      <div class="processes">
        <div class="processes-header">Processes (${gpu.processes.length})</div>
        ${rows}
      </div>`;
  }

  return `
    <div class="gpu-card">
      <div class="gpu-card-header">
        <span class="gpu-name">${escapeHtml(gpu.name)}</span>
        <span class="gpu-index">GPU ${gpu.index}</span>
      </div>
      <div class="metrics-grid">
        <div class="metric">
          <span class="metric-label">Utilization</span>
          <span class="metric-value">${gpu.utilization_gpu}%</span>
          <div class="metric-bar">
            <div class="metric-bar-fill ${utilizationColor(gpu.utilization_gpu)}" style="width: ${gpu.utilization_gpu}%"></div>
          </div>
        </div>
        <div class="metric">
          <span class="metric-label">Temperature</span>
          <span class="metric-value">${temp}°${tempUnit()}</span>
          <div class="metric-bar">
            <div class="metric-bar-fill ${tempColor(gpu.temperature)}" style="width: ${Math.min(gpu.temperature, 100)}%"></div>
          </div>
        </div>
        <div class="metric">
          <span class="metric-label">Memory</span>
          <span class="metric-value">${memPct}%</span>
          <div class="metric-bar">
            <div class="metric-bar-fill bar-accent" style="width: ${memPct}%"></div>
          </div>
          <span class="metric-detail">${formatBytes(gpu.memory_used)} / ${formatBytes(gpu.memory_total)}</span>
        </div>
        <div class="metric">
          <span class="metric-label">Power</span>
          <span class="metric-value">${gpu.power_usage}W</span>
          <div class="metric-bar">
            <div class="metric-bar-fill bar-accent" style="width: ${powerPct}%"></div>
          </div>
          <span class="metric-detail">${gpu.power_usage}W / ${gpu.power_limit}W</span>
        </div>
        <div class="metric">
          <span class="metric-label">Fan</span>
          <span class="metric-value">${gpu.fan_speed !== null ? gpu.fan_speed + '%' : 'N/A'}</span>
          ${gpu.fan_speed !== null ? `
          <div class="metric-bar">
            <div class="metric-bar-fill bar-accent" style="width: ${gpu.fan_speed}%"></div>
          </div>` : ''}
        </div>
        <div class="metric">
          <span class="metric-label">Clocks</span>
          <span class="metric-value">${gpu.clock_gpu} MHz</span>
          <span class="metric-detail">Mem: ${gpu.clock_memory} MHz</span>
        </div>
      </div>
      ${processesHtml}
    </div>`;
}

function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

function renderGpuList(data) {
  const driverInfo = document.getElementById('driver-info');
  driverInfo.textContent = `Driver ${data.driver_version}${data.cuda_version ? ` | CUDA ${data.cuda_version}` : ''}`;

  const gpuList = document.getElementById('gpu-list');
  gpuList.innerHTML = data.gpus.map(renderGpuCard).join('');
}

// Button group helper: get selected value
function getBtnGroupValue(id) {
  const active = document.querySelector(`#${id} .btn-option.active`);
  return active ? active.dataset.value : null;
}

// Button group helper: set selected value
function setBtnGroupValue(id, value) {
  const btns = document.querySelectorAll(`#${id} .btn-option`);
  btns.forEach(btn => {
    btn.classList.toggle('active', btn.dataset.value === value);
  });
}

// Settings management
async function loadSettings() {
  try {
    currentSettings = await invoke('get_settings');
    applySettingsToUI();
  } catch (e) {
    console.error('Failed to load settings:', e);
  }
}

function applySettingsToUI() {
  if (!currentSettings) return;

  const checkboxes = document.querySelectorAll('.checkbox-group input[type="checkbox"]');
  checkboxes.forEach(cb => {
    cb.checked = currentSettings.inline_metrics.includes(cb.value);
  });

  setBtnGroupValue('gpu-mode', currentSettings.inline_gpu_mode);
  setBtnGroupValue('temp-unit', currentSettings.temp_unit);
  setBtnGroupValue('poll-interval', String(currentSettings.poll_interval_ms));

  document.getElementById('single-gpu-select').classList.toggle(
    'hidden', currentSettings.inline_gpu_mode !== 'single'
  );
}

function gatherSettingsFromUI() {
  const checkboxes = document.querySelectorAll('.checkbox-group input[type="checkbox"]:checked');
  const metrics = Array.from(checkboxes).map(cb => cb.value);

  const gpuMode = getBtnGroupValue('gpu-mode');
  const tempUnit = getBtnGroupValue('temp-unit');
  const pollInterval = parseInt(getBtnGroupValue('poll-interval'));
  const gpuIndex = getBtnGroupValue('gpu-index');

  return {
    inline_metrics: metrics.length > 0 ? metrics : ['utilization'],
    inline_gpu_mode: gpuMode,
    selected_gpu_index: gpuMode === 'single' ? parseInt(gpuIndex) || 0 : null,
    poll_interval_ms: pollInterval,
    temp_unit: tempUnit,
  };
}

// Wire up all button groups for click-to-select
document.addEventListener('click', (e) => {
  const btn = e.target.closest('.btn-option');
  if (!btn) return;
  const group = btn.parentElement;
  group.querySelectorAll('.btn-option').forEach(b => b.classList.remove('active'));
  btn.classList.add('active');

  // If gpu-mode changed, show/hide single GPU select
  if (group.id === 'gpu-mode') {
    document.getElementById('single-gpu-select').classList.toggle(
      'hidden', btn.dataset.value !== 'single'
    );
  }
});

// Event listeners
document.getElementById('settings-toggle').addEventListener('click', () => {
  showSettings = !showSettings;
  document.getElementById('settings-panel').classList.toggle('hidden', !showSettings);
  document.getElementById('gpu-list').classList.toggle('hidden', showSettings);
});

document.getElementById('save-settings').addEventListener('click', async () => {
  const settings = gatherSettingsFromUI();
  try {
    await invoke('save_settings', { newSettings: settings });
    currentSettings = settings;
    showSettings = false;
    document.getElementById('settings-panel').classList.add('hidden');
    document.getElementById('gpu-list').classList.remove('hidden');
  } catch (e) {
    console.error('Failed to save settings:', e);
  }
});

// Listen for GPU data events
listen('gpu-data', (event) => {
  const data = event.payload;
  renderGpuList(data);

  // Update GPU index selector (button group)
  const gpuIndexGroup = document.getElementById('gpu-index');
  const currentBtnCount = gpuIndexGroup.querySelectorAll('.btn-option').length;
  if (currentBtnCount !== data.gpus.length) {
    gpuIndexGroup.innerHTML = data.gpus.map((g, i) =>
      `<button class="btn-option${i === 0 ? ' active' : ''}" data-value="${g.index}">GPU ${g.index}</button>`
    ).join('');
  }
});

// Init
loadSettings();
