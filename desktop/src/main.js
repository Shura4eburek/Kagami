const { invoke } = window.__TAURI__.core;
const { getCurrentWindow } = window.__TAURI__.window;
const { listen } = window.__TAURI__.event;

const win = getCurrentWindow();

// ---- window controls (frameless) ----
document.getElementById("min").addEventListener("click", () => win.minimize());
document.getElementById("close").addEventListener("click", () => win.hide()); // в трей

// ---- tab / view switching ----
function showView(name) {
  document.querySelectorAll(".tabs button").forEach((b) =>
    b.setAttribute("aria-selected", String(b.dataset.view === name))
  );
  document.querySelectorAll(".view").forEach((v) =>
    v.classList.toggle("active", v.id === `view-${name}`)
  );
}
document.querySelectorAll(".tabs button").forEach((b) =>
  b.addEventListener("click", () => showView(b.dataset.view))
);
// трей → «Настройки»
listen("nav", (e) => showView(e.payload));

// ---- receiver start/stop ----
const dot = document.getElementById("statusDot");
const title = document.getElementById("statusTitle");
const sub = document.getElementById("statusSub");
const toggle = document.getElementById("toggleReceiver");
let running = false;
let serviceStatus = "idle"; // idle | starting | ready | error

// события готовности сервиса из Rust (рефлектор поднял анонс)
listen("status", (e) => {
  serviceStatus = e.payload;
  render();
});

const footbar = document.getElementById("footbar");
const deviceList = document.getElementById("deviceList");
const devices = {}; // slot -> {name, model}

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c])
  );
}

function tileHTML(d) {
  const isApple = /iphone|ipad|ipod/i.test(d.model + " " + d.name);
  const cls = isApple ? "ios" : "mac";
  const modelTag = d.model ? `<span class="tag">${esc(d.model)}</span>` : "";
  return (
    `<div class="dev"><div class="thumb ${cls}"><span class="live"><i></i>LIVE</span></div>` +
    `<div class="meta"><h3>${esc(d.name || "Устройство")}</h3>` +
    `<div class="row">${modelTag}<span class="tag good">Активно</span></div></div></div>`
  );
}

function emptyHTML() {
  const t = running ? "Ожидание устройства…" : "Пока никого.";
  const x = running
    ? "Открой Пункт управления → Повтор экрана и выбери «Kagami»."
    : "Запусти приёмник — и подключай iPhone или Mac через Повтор экрана.";
  return `<div class="empty"><span class="plus">+</span><p id="emptyTitle">${t}</p><p id="emptyText">${x}</p></div>`;
}

function renderDevices() {
  const slots = Object.keys(devices);
  if (running && slots.length) {
    deviceList.classList.remove("stopped");
    deviceList.innerHTML = slots.map((s) => tileHTML(devices[s])).join("");
  } else {
    deviceList.classList.add("stopped");
    deviceList.innerHTML = emptyHTML();
  }
}

// события об устройствах из uxplay (Rust), по слотам (до 2 одновременно)
listen("device", (e) => {
  const d = e.payload || {};
  if (d.connected) devices[d.slot] = { name: d.name, model: d.model };
  else delete devices[d.slot];
  render();
});

function render() {
  toggle.textContent = running ? "Остановить" : "Запустить";
  toggle.classList.toggle("primary", !running);
  toggle.classList.toggle("ghost", running);

  // индикатор состояния
  const state = !running ? "idle" : serviceStatus === "ready" ? "ready" : serviceStatus === "error" ? "error" : "starting";
  dot.classList.remove("on", "starting", "err");
  if (state === "ready") dot.classList.add("on");
  else if (state === "starting") dot.classList.add("starting");
  else if (state === "error") dot.classList.add("err");

  // hero
  if (state === "idle") {
    title.textContent = "Приёмник остановлен";
    sub.textContent = "Нажми «Запустить», чтобы стать видимым для iPhone и Mac";
  } else if (state === "starting") {
    title.textContent = "Запуск…";
    sub.textContent = "Поднимаем приёмник и анонс в сети — несколько секунд";
  } else if (state === "ready") {
    title.textContent = "Виден в сети как «Kagami»";
    sub.innerHTML = 'AirPlay готов · до 2 устройств · <code>192.168.1.102</code>';
  } else {
    title.textContent = "Ошибка запуска";
    sub.textContent = "Рефлектор не поднялся — проверь Python 3.13 и Bonjour (dns-sd)";
  }

  renderDevices();

  // футер
  const n = Object.keys(devices).length;
  footbar.innerHTML = running
    ? `<span><b>${n}</b> / 2 подключено</span><span class="dot">·</span>` +
      '<span>Декодер <b>Direct3D12</b></span><span class="dot">·</span>' +
      '<span>GPU <b>RTX 4070 Ti</b></span>'
    : '<span>Приёмник не запущен</span><span class="dot">·</span>' +
      '<span>Декодер <b>Direct3D12</b></span><span class="dot">·</span>' +
      '<span>GPU <b>RTX 4070 Ti</b></span>';
}

toggle.addEventListener("click", async () => {
  const name = document.getElementById("deviceName")?.value || "Kagami";
  try {
    if (running) {
      await invoke("stop_receiver");
      running = false;
      serviceStatus = "idle";
    } else {
      serviceStatus = "starting";
      await invoke("start_receiver", { name });
      running = true;
    }
    for (const k of Object.keys(devices)) delete devices[k];
  } catch (err) {
    sub.textContent = "Ошибка: " + err;
  }
  render();
});

// восстановить статус при открытии окна
invoke("receiver_status").then((r) => {
  running = !!r;
  render();
});

// ---- toggles / segments (демо-интерактив) ----
document.querySelectorAll(".sw").forEach((sw) =>
  sw.addEventListener("click", () =>
    sw.setAttribute("aria-checked", sw.getAttribute("aria-checked") === "true" ? "false" : "true")
  )
);
document.querySelectorAll(".segd").forEach((g) =>
  g.querySelectorAll("button").forEach((b) =>
    b.addEventListener("click", () => {
      if (b.disabled) return;
      g.querySelectorAll("button").forEach((x) => x.setAttribute("aria-selected", "false"));
      b.setAttribute("aria-selected", "true");
    })
  )
);
render();
