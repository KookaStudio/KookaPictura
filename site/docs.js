(function () {
  var ROOT = "docs-src/";
  var GITHUB = "https://github.com/KookaStudio/KookaPictura/blob/master/public_docs/";
  var HOME = "README.md";
  var body = document.getElementById("docs-body");
  var ghLink = document.getElementById("docs-github");

  function docPath() {
    var p = decodeURIComponent(location.hash.replace(/^#/, ""));
    return p || HOME;
  }

  function resolve(path, rel) {
    return new URL(rel, new URL(ROOT + path, document.baseURI));
  }

  function inRoot(url) {
    return url.href.indexOf(new URL(ROOT, document.baseURI).href) === 0;
  }

  function rewrite(path) {
    var rootHref = new URL(ROOT, document.baseURI).href;
    body.querySelectorAll("a[href]").forEach(function (a) {
      var href = a.getAttribute("href");
      if (/^([a-z][a-z0-9+.-]*:|\/\/|#)/i.test(href)) return;
      var u = resolve(path, href);
      if (!inRoot(u)) return;
      if (/\.md$/i.test(u.pathname)) a.setAttribute("href", "#" + u.href.slice(rootHref.length));
      else a.setAttribute("href", u.href);
    });
    body.querySelectorAll("img[src]").forEach(function (img) {
      var src = img.getAttribute("src");
      if (/^([a-z][a-z0-9+.-]*:|\/\/|data:)/i.test(src)) return;
      var u = resolve(path, src);
      if (inRoot(u)) img.setAttribute("src", u.href);
    });
  }

  var nav = document.getElementById("docs-nav");
  var toc = document.getElementById("docs-toc");

  function buildToc(md) {
    var ul = null;
    md.split("\n").forEach(function (line) {
      var h = /^##\s+(.+?)\s*$/.exec(line);
      if (h) {
        var head = document.createElement("p");
        head.className = "docs-toc-head";
        head.textContent = h[1];
        nav.appendChild(head);
        ul = document.createElement("ul");
        nav.appendChild(ul);
        return;
      }
      var m = /^[-*]\s+\[([^\]]+)\]\(([^)#:]+\.md)\)/.exec(line);
      if (!m) return;
      if (!ul) {
        ul = document.createElement("ul");
        nav.appendChild(ul);
      }
      var a = document.createElement("a");
      a.href = "#" + m[2];
      a.textContent = m[1];
      ul.appendChild(document.createElement("li")).appendChild(a);
    });
    markCurrent();
  }

  function markCurrent() {
    var path = docPath();
    nav.querySelectorAll("a").forEach(function (a) {
      var on = decodeURIComponent(a.getAttribute("href").slice(1)) === path;
      if (on) a.setAttribute("aria-current", "page"); else a.removeAttribute("aria-current");
    });
  }

  function pct(v, total) { return (v / total * 100) + "%"; }

  function hotspots(img, data) {
    var wrap = document.createElement("div");
    wrap.className = "hs-wrap";
    img.parentNode.insertBefore(wrap, img);
    wrap.appendChild(img);
    var tip = document.createElement("div");
    tip.className = "hs-tip";
    tip.hidden = true;
    wrap.appendChild(tip);
    var areas = {};

    function show(n, x, y) {
      Object.keys(areas).forEach(function (k) { areas[k].classList.toggle("on", Number(k) === n); });
      var p = data.points.filter(function (q) { return q.n === n; })[0];
      tip.textContent = p.n + ". " + p.label;
      tip.style.left = pct(x, data.width);
      tip.style.top = pct(y, data.height);
      tip.classList.toggle("below", y < 90);
      tip.hidden = false;
    }
    function hide() {
      Object.keys(areas).forEach(function (k) { areas[k].classList.remove("on"); });
      tip.hidden = true;
    }

    data.points.forEach(function (p) {
      var r = p.rect;
      var area = document.createElement("div");
      area.className = "hs-area";
      area.style.left = pct(r[0], data.width);
      area.style.top = pct(r[1], data.height);
      area.style.width = pct(r[2] - r[0], data.width);
      area.style.height = pct(r[3] - r[1], data.height);
      areas[p.n] = area;
      wrap.appendChild(area);

      var badge = document.createElement("button");
      badge.type = "button";
      badge.className = "hs-badge";
      badge.setAttribute("aria-label", p.label);
      badge.style.left = pct(p.badge[0], data.width);
      badge.style.top = pct(p.badge[1], data.height);
      wrap.appendChild(badge);

      area.addEventListener("mousemove", function (e) {
        var box = wrap.getBoundingClientRect();
        show(p.n, (e.clientX - box.left) / box.width * data.width, (e.clientY - box.top) / box.height * data.height);
      });
      area.addEventListener("mouseleave", hide);
      badge.addEventListener("mouseenter", function () { show(p.n, p.badge[0], p.badge[1]); });
      badge.addEventListener("focus", function () { show(p.n, p.badge[0], p.badge[1]); });
      badge.addEventListener("mouseleave", hide);
      badge.addEventListener("blur", hide);
      badge.addEventListener("click", function () { show(p.n, p.badge[0], p.badge[1]); });
    });
  }

  var ICON_MAX = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M15 3h6v6"/><path d="M9 21H3v-6"/><path d="M21 3l-7 7"/><path d="M3 21l7-7"/></svg>';
  var ICON_MIN = '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M14 10h6V4"/><path d="M10 14H4v6"/><path d="M20 4l-6 6"/><path d="M4 20l6-6"/></svg>';
  var viewer = null;

  function closeViewer() {
    if (!viewer) return;
    viewer.remove();
    viewer = null;
    document.body.style.overflow = "";
  }

  function openViewer(img) {
    closeViewer();
    viewer = document.createElement("div");
    viewer.className = "img-viewer";
    viewer.setAttribute("role", "dialog");
    viewer.setAttribute("aria-label", img.alt || "Image");
    var full = document.createElement("img");
    full.src = img.currentSrc || img.src;
    full.alt = img.alt;
    var btn = document.createElement("button");
    btn.type = "button";
    btn.className = "img-expand";
    btn.setAttribute("aria-label", "Minimize image");
    btn.innerHTML = ICON_MIN;
    btn.addEventListener("click", closeViewer);
    viewer.addEventListener("click", function (e) { if (e.target === viewer) closeViewer(); });
    viewer.appendChild(full);
    viewer.appendChild(btn);
    document.body.appendChild(viewer);
    document.body.style.overflow = "hidden";
    btn.focus();
  }

  function addExpand(wrap, img) {
    var btn = document.createElement("button");
    btn.type = "button";
    btn.className = "img-expand";
    btn.setAttribute("aria-label", "Expand image to full size");
    btn.innerHTML = ICON_MAX;
    btn.addEventListener("click", function () { openViewer(img); });
    wrap.appendChild(btn);
  }

  function plainWrap(img) {
    if (!img.parentNode || img.closest(".hs-wrap, .img-wrap")) return;
    var wrap = document.createElement("div");
    wrap.className = "img-wrap";
    img.parentNode.insertBefore(wrap, img);
    wrap.appendChild(img);
    addExpand(wrap, img);
  }

  function addHotspots() {
    body.querySelectorAll("img").forEach(function (img) {
      var src = img.getAttribute("src");
      if (!/\.png$/i.test(src)) { plainWrap(img); return; }
      fetch(src.replace(/\.png$/i, ".hotspots.json"))
        .then(function (r) { return r.ok ? r.json() : null; })
        .then(function (data) {
          if (!img.parentNode) return;
          if (data) {
            hotspots(img, data);
            addExpand(img.parentNode, img);
          } else {
            plainWrap(img);
          }
        })
        .catch(function () { plainWrap(img); });
    });
  }

  document.addEventListener("keydown", function (e) {
    if (e.key === "Escape") closeViewer();
  });

  function load() {
    markCurrent();
    var path = docPath();
    ghLink.href = GITHUB + path;
    if (!inRoot(resolve("", path))) {
      body.textContent = "Page not found.";
      return;
    }
    fetch(ROOT + path)
      .then(function (r) {
        if (!r.ok) throw new Error(String(r.status));
        return r.text();
      })
      .then(function (md) {
        body.innerHTML = marked.parse(md);
        rewrite(path);
        addHotspots();
        document.title = (body.querySelector("h1") || { textContent: path }).textContent + " — Kooka Pictura docs";
        window.scrollTo(0, 0);
      })
      .catch(function () {
        body.textContent = "This page could not be loaded. ";
        var a = document.createElement("a");
        a.href = GITHUB + path;
        a.textContent = "Open it on GitHub.";
        body.appendChild(a);
      });
  }

  window.addEventListener("hashchange", load);
  fetch(ROOT + HOME)
    .then(function (r) { return r.ok ? r.text() : ""; })
    .then(buildToc);
  if (window.matchMedia("(max-width: 800px)").matches) toc.open = false;
  load();
})();
