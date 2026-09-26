const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let snapshot;
let selectedSource = "microphone";
let page = "capture";
let errorMessage = "";
let modelEntries = [];
let confirmRemoveModel = false;
let recordingEntries = [];
let recordingDetails = null;
let selectedRecordingId = null;
let confirmDeleteRecording = false;
let liveTranscriptUpdates = new Map();
let audioSource = "microphone";
let liveCacheSessionId = null;

const pages = [
  ["capture", "Capture"],
  ["models", "Models"],
  ["recordings", "Recordings"],
  ["settings", "Settings"],
];

function duration(milliseconds = 0) {
  const seconds = Math.floor(milliseconds / 1000);
  return `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
}

function activeSource() {
  if (!snapshot) return selectedSource;
  const mic = snapshot.microphone.enabled;
  const system = snapshot.system_audio.enabled;
  selectedSource = mic && system ? "both" : system ? "system" : "microphone";
  return selectedSource;
}

function render() {
  if (!snapshot) return;
  const recording = snapshot.status === "recording";
  const source = activeSource();
  const stateText = recording ? "Listening" : snapshot.status === "starting" ? "Starting" : "Ready";
  document.querySelector("#app").innerHTML = `
    <header class="title-row">
      <div class="brand-mark" aria-hidden="true">≋</div>
      <div><h1>RimV</h1><p>Local transcription</p></div>
      <span class="state ${recording ? "live" : ""}"><span class="state-dot"></span>${stateText}</span>
    </header>
    <nav class="nav" aria-label="RimV sections">
      ${pages.map(([id, label]) => `<button class="nav-item ${page === id ? "selected" : ""}" data-page="${id}">${label}</button>`).join("")}
    </nav>
    ${page === "capture" ? `
      <section class="content" aria-labelledby="capture-title">
        <div class="section-heading"><div><p class="eyebrow">AUDIO INPUT</p><h2 id="capture-title">Capture source</h2></div><span class="timer">${duration(snapshot.elapsed_ms)}</span></div>
        <div class="source-grid" role="group" aria-label="Capture source">
          ${[["system", "System audio", "Playback from your selected output"], ["microphone", "Microphone", "Input from your selected microphone"], ["both", "Both", "Keep both sources in separate tracks"]].map(([id, title, detail]) => `<button class="source-card ${source === id ? "chosen" : ""}" data-source="${id}" aria-pressed="${source === id}"><span class="source-check" aria-hidden="true">${source === id ? "✓" : ""}</span><strong>${title}</strong><span>${detail}</span></button>`).join("")}
        </div>
        <button id="listen" class="primary ${recording ? "stop" : ""}">${recording ? "Stop listening" : "Start listening"}</button>
        <p class="capture-hint">${recording ? "Capture continues while this window is closed." : "Audio is processed locally on this device."}</p>
        <div class="detail-row"><span>Microphone</span><b>${snapshot.microphone.active ? "Active" : snapshot.microphone.enabled ? "Selected" : "Off"}</b></div>
        <div class="detail-row"><span>System audio</span><b>${snapshot.system_audio.active ? "Active" : snapshot.system_audio.enabled ? "Selected" : "Off"}</b></div>
        <div class="detail-row"><span>Transcription</span><b>${snapshot.transcription.available ? snapshot.transcription.status : "No model installed"}</b></div>
        ${errorMessage ? `<p class="error" role="alert">${escapeText(errorMessage)}</p>` : ""}
      </section>
    ` : `
      <section class="content secondary-page">
        <p class="eyebrow">${pages.find(([id]) => id === page)?.[1].toUpperCase()}</p>
        <h2>${pages.find(([id]) => id === page)?.[1]}</h2>
        ${page === "models" ? renderModelsPage() : page === "recordings" ? renderRecordingsPage() : `
          <p>Capture continues while the window is hidden. Audio and transcripts are stored locally.</p>
          <button class="secondary" data-page="capture">Back to capture</button>
        `}
      </section>
    `}
    <footer><span>RimV runs in the notification area</span><button id="quit" class="text-button">Quit</button></footer>
  `;
  bind();
}

function escapeText(text) {
  const element = document.createElement("span");
  element.textContent = text;
  return element.innerHTML;
}

function bind() {
  document.querySelectorAll("[data-page]").forEach(button => button.addEventListener("click", () => {
    page = button.dataset.page;
    render();
    if (page === "models") refreshModels();
    if (page === "recordings") refreshRecordings();
  }));
  document.querySelectorAll("[data-source]").forEach(button => button.addEventListener("click", async () => {
    selectedSource = button.dataset.source;
    errorMessage = "";
    render();
    try {
      snapshot = await invoke("set_capture_source", { source: selectedSource });
    } catch (error) {
      errorMessage = String(error);
      snapshot = await invoke("get_state");
    }
    render();
  }));
  document.querySelector("#listen")?.addEventListener("click", async () => {
    errorMessage = "";
    try {
      snapshot = await invoke("set_listening", { enabled: snapshot.status !== "recording" });
    } catch (error) {
      errorMessage = String(error);
      snapshot = await invoke("get_state");
    }
    render();
  });
  document.querySelector("#quit")?.addEventListener("click", () => invoke("quit_app"));
  document.querySelectorAll("[data-recording]").forEach(button => button.addEventListener("click", () => openRecording(button.dataset.recording)));
  document.querySelector("#recordings-refresh")?.addEventListener("click", refreshRecordings);
  document.querySelector("#recording-back")?.addEventListener("click", () => { selectedRecordingId = null; recordingDetails = null; render(); });
  document.querySelector("#recording-rename")?.addEventListener("click", async () => {
    const input = document.querySelector("#recording-title");
    errorMessage = "";
    try {
      await invoke("rename_recording", { sessionId: selectedRecordingId, title: input.value });
      await refreshRecordings();
      await openRecording(selectedRecordingId);
    } catch (error) { errorMessage = String(error); render(); }
  });
  document.querySelector("#recording-delete")?.addEventListener("click", async () => {
    if (!confirmDeleteRecording) { confirmDeleteRecording = true; render(); return; }
    errorMessage = "";
    try {
      await invoke("delete_recording", { sessionId: selectedRecordingId });
      selectedRecordingId = null;
      recordingDetails = null;
      confirmDeleteRecording = false;
      await refreshRecordings();
    } catch (error) { errorMessage = String(error); render(); }
  });
  document.querySelector("#recording-delete-cancel")?.addEventListener("click", () => { confirmDeleteRecording = false; render(); });
  document.querySelector("#audio-source")?.addEventListener("change", event => { audioSource = event.target.value; render(); });
  document.querySelector("#export-txt")?.addEventListener("click", () => exportTranscript("txt"));
  document.querySelector("#export-json")?.addEventListener("click", () => exportTranscript("json"));
  document.querySelector("#install-model")?.addEventListener("click", async () => {
    await runModelAction(() => invoke("install_model", { modelId: modelEntries[0].id }));
  });
  document.querySelector("#cancel-model")?.addEventListener("click", async () => {
    await runModelAction(() => invoke("cancel_model_install", { modelId: modelEntries[0].id }));
  });
  document.querySelector("#select-model")?.addEventListener("click", async () => {
    await runModelAction(async () => {
      snapshot = await invoke("select_model", { modelId: modelEntries[0].id });
    });
  });
  document.querySelector("#clear-model")?.addEventListener("click", async () => {
    await runModelAction(async () => {
      snapshot = await invoke("clear_selected_model");
    });
  });
  document.querySelector("#remove-model")?.addEventListener("click", async () => {
    if (!confirmRemoveModel) {
      confirmRemoveModel = true;
      render();
      return;
    }
    await runModelAction(async () => {
      const model = modelEntries[0];
      if (model.selected) await invoke("clear_selected_model");
      await invoke("remove_model", { modelId: model.id });
    });
  });
  document.querySelector("#cancel-remove-model")?.addEventListener("click", () => {
    confirmRemoveModel = false;
    render();
  });
  document.querySelector("#language")?.addEventListener("change", async event => {
    await runModelAction(async () => {
      snapshot = await invoke("select_language", {
        language: event.target.value === "auto" ? null : event.target.value,
      });
    });
  });
}

function renderRecordingsPage() {
  if (!selectedRecordingId || !recordingDetails) {
    return `
      <div class="recordings-heading"><p class="muted">Saved sessions from this device</p><button id="recordings-refresh" class="text-button">Refresh</button></div>
      ${recordingEntries.length ? `<div class="recording-list">${recordingEntries.map(item => `
        <button class="recording-card" data-recording="${escapeText(item.session_id)}">
          <span class="recording-card-top"><strong>${escapeText(item.title)}</strong><span class="model-state ${item.state === "recording" ? "selected" : ""}">${item.state === "recording" ? "Recording" : "Completed"}</span></span>
          <span>${new Date(item.started_at_unix_ms).toLocaleString()} · ${duration(item.duration_ms)}</span>
          <span>${item.sources.length ? item.sources.map(sourceLabel).join(" + ") : "No audio frames saved"}${item.has_transcript ? " · Transcript" : ""}</span>
        </button>`).join("")}</div>` : `<p class="empty-state">${snapshot.status === "recording" ? "The current recording will appear here." : "No recordings saved yet."}</p>`}
      ${errorMessage ? `<p class="error" role="alert">${escapeText(errorMessage)}</p>` : ""}
    `;
  }
  const summary = recordingDetails.summary;
  const live = summary.state === "recording";
  const sources = summary.sources;
  if (!sources.includes(audioSource)) audioSource = sources[0] || "microphone";
  const audioSrc = sources.includes(audioSource) ? `rimv-audio://localhost/${encodeURIComponent(summary.session_id)}/${audioSource}` : "";
  return `
    <div class="recording-view-heading"><button id="recording-back" class="text-button">← Recordings</button><span class="model-state ${live ? "selected" : ""}">${live ? "Live transcript" : "Completed"}</span></div>
    <h2 class="recording-title">${escapeText(summary.title)}</h2>
    <p class="muted small-note">${new Date(summary.started_at_unix_ms).toLocaleString()} · <span class="recording-duration">${duration(snapshot.status === "recording" && live ? snapshot.elapsed_ms : summary.duration_ms)}</span></p>
    <div class="rename-row"><label class="sr-only" for="recording-title">Recording name</label><input id="recording-title" maxlength="120" value="${escapeText(summary.title)}"><button id="recording-rename" class="secondary compact">Save name</button></div>
    ${live ? `<p class="muted small-note">Audio playback is available after capture stops.</p>` : sources.length ? `<section class="audio-panel"><label class="field-label" for="audio-source">Audio track</label><select id="audio-source" class="language-select">${sources.map(source => `<option value="${source}" ${source === audioSource ? "selected" : ""}>${sourceLabel(source)}</option>`).join("")}</select><audio controls preload="metadata" src="${audioSrc}" aria-label="Recording audio"></audio></section>` : `<p class="empty-state">No audio track was saved for this session.</p>`}
    <div class="section-heading transcript-heading"><div><p class="eyebrow">${live ? "UPDATING WHILE CAPTURE CONTINUES" : "SAVED LOCALLY"}</p><h3>Transcript</h3></div><div class="export-actions"><button id="export-txt" class="text-button" ${recordingDetails.transcript.length ? "" : "disabled"}>TXT</button><button id="export-json" class="text-button" ${recordingDetails.transcript.length ? "" : "disabled"}>JSON</button></div></div>
    <div id="transcript-lines" class="transcript-lines">${renderTranscriptLines()}</div>
    <div class="recording-actions"><button id="recording-delete" class="text-button destructive">${confirmDeleteRecording ? "Confirm delete recording" : "Delete recording"}</button>${confirmDeleteRecording ? `<button id="recording-delete-cancel" class="text-button">Cancel</button>` : ""}</div>
    ${errorMessage ? `<p class="error" role="alert">${escapeText(errorMessage)}</p>` : ""}
  `;
}

function sourceLabel(source) { return source === "system" ? "System audio" : "Microphone"; }

function transcriptLines() {
  if (recordingDetails?.summary.state === "recording" && liveTranscriptUpdates.size) {
    return [...liveTranscriptUpdates.values()].map(update => ({
      source: update.source,
      start_ms: update.start_ms,
      end_ms: update.end_ms,
      text: [update.stable_text, update.unstable_text].filter(Boolean).join(" "),
      partial: !update.is_final,
    })).filter(line => line.text.trim()).sort((a, b) => a.start_ms - b.start_ms);
  }
  return (recordingDetails?.transcript || []).map(line => ({ ...line, partial: false }));
}

function renderTranscriptLines() {
  const lines = transcriptLines();
  return lines.length ? lines.map(line => `<article class="transcript-line ${line.partial ? "partial" : ""}"><div><span>${sourceLabel(line.source)}</span><time>${duration(line.start_ms)}</time>${line.partial ? `<span class="partial-label">In progress</span>` : ""}</div><p>${escapeText(line.text)}</p></article>`).join("") : `<p class="empty-state">${snapshot.status === "recording" && recordingDetails?.summary.state === "recording" ? "Listening for speech…" : "No transcript was saved for this session."}</p>`;
}

async function refreshRecordings() {
  try { recordingEntries = await invoke("get_recordings"); errorMessage = ""; }
  catch (error) { errorMessage = String(error); }
  render();
}

async function openRecording(sessionId) {
  selectedRecordingId = sessionId;
  recordingDetails = null;
  liveTranscriptUpdates.clear();
  render();
  try {
    recordingDetails = await invoke("get_recording", { sessionId });
    if (recordingDetails.summary.state === "recording") {
      const live = await invoke("get_live_transcript");
      if (live.session_id === sessionId) live.updates.forEach(update => liveTranscriptUpdates.set(`${update.source}:${update.utterance_id}`, update));
    }
    errorMessage = "";
  } catch (error) { errorMessage = String(error); }
  render();
}

function exportTranscript(format) {
  const lines = transcriptLines();
  if (!lines.length || !recordingDetails) return;
  const body = format === "json"
    ? JSON.stringify(lines.map(({ partial, ...line }) => line), null, 2)
    : lines.map(line => `[${line.start_ms}–${line.end_ms} ms] ${sourceLabel(line.source)}: ${line.text}`).join("\n") + "\n";
  const blob = new Blob([body], { type: format === "json" ? "application/json" : "text/plain" });
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = `rimv-${recordingDetails.summary.session_id}.${format}`;
  link.click();
  setTimeout(() => URL.revokeObjectURL(link.href), 1000);
}

function renderModelsPage() {
  if (!modelEntries.length) {
    return `<p id="phase-note" class="muted">Loading installed model state…</p>`;
  }
  const model = modelEntries[0];
  const installed = model.state === "installed";
  const downloading = model.state === "downloading";
  const extracting = model.state === "extracting";
  const selected = installed && model.selected;
  const canChange = snapshot.status === "idle";
  const progressText = model.progress_total_bytes
    ? `${(model.progress_bytes / 1_000_000).toFixed(1)} / ${(model.progress_total_bytes / 1_000_000).toFixed(1)} MB`
    : `${(model.progress_bytes / 1_000_000).toFixed(1)} MB`;
  const status = downloading
    ? `Downloading · ${model.progress_total_bytes ? `${Math.floor(model.progress_bytes * 100 / model.progress_total_bytes)}%` : progressText}`
    : extracting ? "Installing files"
    : selected ? "Selected" : model.state === "installed" ? "Installed" : model.state[0].toUpperCase() + model.state.slice(1);
  const languageControl = selected ? `
    <label class="field-label" for="language">Transcription language</label>
    <select id="language" class="language-select" ${canChange ? "" : "disabled"}>
      <option value="auto" ${model.selected_language ? "" : "selected"}>Auto-detect</option>
      ${model.languages.map(code => `<option value="${escapeText(code)}" ${model.selected_language === code ? "selected" : ""}>${escapeText(languageName(code))}</option>`).join("")}
    </select>
    <p class="muted small-note">Languages come from this model’s catalog metadata.</p>
  ` : "";
  let actions;
  if (downloading) {
    actions = `<button id="cancel-model" class="secondary">Cancel download</button>`;
  } else if (extracting) {
    actions = `<p class="muted">Installing model files. Keep RimV open until this finishes.</p>`;
  } else if (selected) {
    actions = `
      ${languageControl}
      <button id="clear-model" class="secondary" ${canChange ? "" : "disabled"}>Disable transcription</button>
      ${renderRemoveModelAction(canChange)}
    `;
  } else if (installed) {
    actions = `
      <button id="select-model" class="primary" ${canChange ? "" : "disabled"}>Use this model</button>
      ${renderRemoveModelAction(canChange)}
    `;
  } else {
    actions = `
      <p class="muted">${model.state === "failed" ? escapeText(model.error || "Installation failed.") : "Downloads the model package and checks its required files."}</p>
      <button id="install-model" class="primary">${model.state === "failed" || model.state === "incomplete" ? "Retry download" : "Download model"}</button>
    `;
  }
  return `
    <article class="model-card">
      <div class="model-heading">
        <div><h3>${escapeText(model.name)}</h3><p class="muted">${escapeText(model.version)} · Parakeet</p></div>
        <span class="model-state ${selected ? "selected" : ""}">${escapeText(status)}</span>
      </div>
      ${selected ? `<p class="muted small-note">Transcription engine: ${escapeText(snapshot.transcription.status)}</p>` : ""}
      ${downloading ? `<progress aria-label="Model download progress" ${model.progress_total_bytes ? `max="${model.progress_total_bytes}" value="${model.progress_bytes}"` : ""}></progress><p class="muted small-note">${escapeText(progressText)} received</p>` : ""}
      ${extracting ? `<progress aria-label="Installing model files"></progress>` : ""}
      ${model.supports_language_detection && installed ? `<p class="muted">This model supports automatic language detection and ${model.languages.length} catalogued languages.</p>` : ""}
      ${actions}
    </article>
    <p class="muted small-note">Only model backends supported by this Windows build are shown.</p>
    ${errorMessage ? `<p class="error" role="alert">${escapeText(errorMessage)}</p>` : ""}
  `;
}

function renderRemoveModelAction(canChange) {
  const label = confirmRemoveModel ? "Confirm model removal" : "Remove downloaded model";
  return `<div class="remove-actions"><button id="remove-model" class="text-button" ${canChange ? "" : "disabled"}>${label}</button>${confirmRemoveModel ? `<button id="cancel-remove-model" class="text-button">Cancel</button>` : ""}</div>`;
}

function languageName(code) {
  try {
    return new Intl.DisplayNames([navigator.language || "en"], { type: "language" }).of(code) || code;
  } catch {
    return code;
  }
}

async function refreshModels() {
  try {
    modelEntries = await invoke("get_model_catalog");
    render();
  } catch (error) {
    errorMessage = String(error);
    render();
  }
}

async function runModelAction(action) {
  errorMessage = "";
  try {
    await action();
    modelEntries = await invoke("get_model_catalog");
  } catch (error) {
    errorMessage = String(error);
    try { modelEntries = await invoke("get_model_catalog"); } catch {}
  }
  confirmRemoveModel = false;
  render();
}

listen("engine-event", event => {
  const value = event.payload;
  if (value.type === "snapshot") {
    const previousStatus = snapshot?.status;
    const previousSessionId = snapshot?.session?.id || null;
    const nextSessionId = value.snapshot.session?.id || null;
    if (nextSessionId !== liveCacheSessionId) {
      liveCacheSessionId = nextSessionId;
      liveTranscriptUpdates.clear();
    }
    snapshot = value.snapshot;
    const stateChanged = previousStatus !== snapshot.status || previousSessionId !== nextSessionId;
    if (page === "recordings" && stateChanged) {
      if (selectedRecordingId && recordingDetails) {
        if (recordingDetails.summary.session_id === nextSessionId) {
          if (["starting", "recording", "stopping"].includes(snapshot.status)) {
            recordingDetails.summary.duration_ms = snapshot.elapsed_ms;
            const timer = document.querySelector(".recording-duration");
            if (timer) timer.textContent = duration(snapshot.elapsed_ms);
          } else {
            openRecording(selectedRecordingId);
          }
        } else {
          refreshRecordings();
        }
      } else {
        refreshRecordings();
      }
      return;
    }
    if (page === "recordings") {
      if (selectedRecordingId && recordingDetails?.summary.state === "recording" && snapshot.status === "recording") {
        recordingDetails.summary.duration_ms = snapshot.elapsed_ms;
        const timer = document.querySelector(".recording-duration");
        if (timer) timer.textContent = duration(snapshot.elapsed_ms);
      }
      return;
    }
    if (page !== "capture" && !stateChanged) return;
  }
  if (value.type === "error" || value.type === "transcription_error") errorMessage = value.error.message;
  if (value.type === "transcript_update") {
    if (snapshot?.session?.id) liveTranscriptUpdates.set(`${value.update.source}:${value.update.utterance_id}`, value.update);
    if (page === "recordings" && selectedRecordingId === snapshot?.session?.id && recordingDetails?.summary.state === "recording") {
      const transcript = document.querySelector("#transcript-lines");
      if (transcript) transcript.innerHTML = renderTranscriptLines();
    }
    return;
  }
  render();
});
listen("desktop-error", event => {
  errorMessage = String(event.payload);
  render();
});
listen("model-progress", event => {
  const progress = event.payload;
  const model = modelEntries.find(item => item.id === progress.model_id);
  if (model) {
    model.progress_bytes = progress.downloaded_bytes;
    model.progress_total_bytes = progress.total_bytes;
    if (progress.phase === "failed") {
      model.state = "failed";
      model.error = progress.error;
    } else if (progress.phase === "complete") {
      model.state = "installed";
    } else if (progress.phase === "extracting") {
      model.state = "extracting";
    } else if (progress.phase === "cancelled") {
      model.state = "available";
    }
    render();
  }
  if (["complete", "failed", "cancelled"].includes(progress.phase)) refreshModels();
});
invoke("get_state").then(value => {
  snapshot = value;
  render();
}).catch(error => {
  document.querySelector("#app").innerHTML = `<p class="error" role="alert">RimV could not start: ${escapeText(String(error))}</p>`;
});
