(function () {
  var REPO = "KookaStudio/KookaPictura";
  var LATEST_PAGE = "https://github.com/" + REPO + "/releases/latest";
  var API = "https://api.github.com/repos/" + REPO + "/releases/latest";

  var LINUX = [
    ["rpm", /\.rpm$/i],
    ["deb", /\.deb$/i],
    ["AppImage", /\.AppImage$/i],
    ["snap", /\.snap$/i],
    ["flatpak", /\.flatpak$/i]
  ];

  var NOTES = {
    macos: [
      "To install Kooka Pictura on macOS:",
      "Open System Settings.",
      "Click Privacy & Security, scroll down, and click the Open Anyway button to confirm your intent to open or install the app.",
      "Click Open.",
      "The app is now saved as an exception to your security settings, and you can open it in the future by double-clicking it, just as you can any authorized app."
    ],
    windows: [
      "To install Kooka Pictura on Windows:",
      "Open Settings by clicking the Start menu and selecting the gear icon.",
      "In the Settings menu, select Apps.",
      "Click Apps & features in the left-hand menu.",
      "Scroll down to the Installing apps option and click it.",
      "Under Installing apps, select Allow apps from anywhere to enable installations from sources other than the Microsoft Store."
    ]
  };

  function find(assets, re) {
    for (var i = 0; i < assets.length; i++) {
      if (re.test(assets[i].name)) return assets[i];
    }
    return null;
  }

  function pickAssets(assets) {
    assets = assets || [];
    var linux = [];
    LINUX.forEach(function (f) {
      var a = find(assets, f[1]);
      if (a) linux.push({ label: f[0], name: a.name, url: a.browser_download_url });
    });
    var dmg = find(assets, /\.dmg$/i);
    var exe = find(assets, /-setup\.exe$/i);
    return {
      linux: linux,
      macos: dmg ? { label: "dmg", name: dmg.name, url: dmg.browser_download_url } : null,
      windows: exe ? { label: "exe", name: exe.name, url: exe.browser_download_url } : null
    };
  }

  if (typeof module !== "undefined" && module.exports) {
    module.exports = { pickAssets: pickAssets };
  }
  if (typeof document === "undefined") return;

  function el(tag, cls, text) {
    var n = document.createElement(tag);
    if (cls) n.className = cls;
    if (text) n.textContent = text;
    return n;
  }

  function link(item) {
    var a = el("a", "dl-link", item.label);
    a.href = item.url;
    a.setAttribute("download", item.name);
    var small = el("span", "dl-file", item.name);
    var li = el("li");
    li.appendChild(a);
    li.appendChild(small);
    return li;
  }

  function fallback() {
    var a = el("a", "dl-link", "Download from GitHub");
    a.href = LATEST_PAGE;
    return a;
  }

  function notes(lines) {
    var box = el("div", "dl-notes");
    box.appendChild(el("p", "dl-notes-title", lines[0]));
    var ol = el("ol");
    lines.slice(1).forEach(function (t) { ol.appendChild(el("li", null, t)); });
    box.appendChild(ol);
    box.appendChild(el("div", "dl-shot"));
    box.lastChild.textContent = "Screenshot coming soon";
    return box;
  }

  function panel(os, picked) {
    var p = el("div", "dl-panel");
    p.hidden = true;
    p.dataset.os = os;
    var items = os === "linux" ? picked.linux : (picked[os] ? [picked[os]] : []);
    if (!items.length) {
      p.appendChild(fallback());
      return p;
    }
    var ul = el("ul", "dl-list");
    items.forEach(function (i) { ul.appendChild(link(i)); });
    p.appendChild(ul);
    if (NOTES[os]) p.appendChild(notes(NOTES[os]));
    return p;
  }

  function loadRelease() {
    try {
      var cached = sessionStorage.getItem("kp-release");
      if (cached) return Promise.resolve(JSON.parse(cached));
    } catch (e) {}
    return fetch(API, { headers: { Accept: "application/vnd.github+json" } })
      .then(function (r) {
        if (!r.ok) throw new Error("release lookup " + r.status);
        return r.json();
      })
      .then(function (rel) {
        try { sessionStorage.setItem("kp-release", JSON.stringify(rel)); } catch (e) {}
        return rel;
      });
  }

  function init(root) {
    var toggle = root.querySelector(".dl-toggle");
    var choices = root.querySelector(".dl-choices");
    var panels = root.querySelector(".dl-panels");
    var release = null;
    var started = false;

    function show(os) {
      choices.querySelectorAll("button").forEach(function (b) {
        b.setAttribute("aria-pressed", String(b.dataset.os === os));
      });
      panels.querySelectorAll(".dl-panel").forEach(function (p) {
        p.hidden = p.dataset.os !== os;
      });
    }

    function render() {
      panels.textContent = "";
      var picked = release ? pickAssets(release.assets) : { linux: [], macos: null, windows: null };
      ["linux", "macos", "windows"].forEach(function (os) { panels.appendChild(panel(os, picked)); });
      var tag = root.querySelector(".dl-tag");
      if (tag) tag.textContent = release && release.tag_name ? "Latest release: " + release.tag_name : "";
    }

    toggle.addEventListener("click", function () {
      var open = choices.hidden;
      choices.hidden = !open;
      toggle.setAttribute("aria-expanded", String(open));
      if (open && !started) {
        started = true;
        loadRelease().then(function (r) { release = r; }, function () { release = null; })
          .then(render);
      }
    });

    choices.addEventListener("click", function (e) {
      var b = e.target.closest("button[data-os]");
      if (!b) return;
      if (!panels.children.length) render();
      show(b.dataset.os);
    });
  }

  document.querySelectorAll("[data-download]").forEach(init);
})();
