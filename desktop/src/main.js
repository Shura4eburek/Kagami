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

const footbar = document.getElementById("footbar");
const deviceList = document.getElementById("deviceList");
const emptyTitle = document.getElementById("emptyTitle");
const emptyText = document.getElementById("emptyText");

function render() {
  dot.classList.toggle("on", running);
  toggle.textContent = running ? "Остановить" : "Запустить";
  toggle.classList.toggle("primary", !running);
  toggle.classList.toggle("ghost", running);

  // hero
  if (running) {
    title.textContent = "Виден в сети как «Kagami»";
    sub.innerHTML = 'AirPlay готов к приёму · <code>192.168.1.102</code>';
  } else {
    title.textContent = "Приёмник остановлен";
    sub.textContent = "Нажми «Запустить», чтобы стать видимым для iPhone и Mac";
  }

  // демо-устройства и «слот» показываем только при работе
  document.querySelectorAll(".dev.demo").forEach((d) => (d.style.display = running ? "flex" : "none"));
  deviceList.classList.toggle("stopped", !running);
  if (running) {
    emptyTitle.textContent = "Свободный слот.";
    emptyText.textContent = "Открой Пункт управления → Повтор экрана и выбери «Kagami».";
  } else {
    emptyTitle.textContent = "Пока никого.";
    emptyText.textContent = "Запусти приёмник — и подключай iPhone или Mac через Повтор экрана.";
  }

  // футер
  footbar.innerHTML = running
    ? '<span>Декодер <b>Direct3D12</b></span><span class="dot">·</span>' +
      '<span>GPU <b>RTX 4070 Ti</b></span><span class="dot">·</span>' +
      '<span><b>118</b> fps суммарно</span><span class="dot">·</span>' +
      '<span>Низкая задержка <b>вкл.</b></span>'
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
    } else {
      await invoke("start_receiver", { name });
      running = true;
    }
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
document.querySelectorAll(".disc").forEach((d) =>
  d.addEventListener("click", () => {
    const dev = d.closest(".dev");
    dev.style.transition = "opacity .25s, transform .25s";
    dev.style.opacity = "0";
    dev.style.transform = "scale(.97)";
    setTimeout(() => dev.remove(), 260);
  })
);

render();
