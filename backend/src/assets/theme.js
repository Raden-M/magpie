// Light/dark toggle. Loaded first in <head>, before the stylesheet, so a stored
// choice is stamped on <html> before the CSS resolves - no flash of the wrong
// theme. The button itself is progressive: it ships `hidden` (a toggle that
// cannot toggle is worse than none) and this script reveals it, exactly as the
// refresh control in the dock does.
(function () {
  "use strict";
  var KEY = "magpie-theme";
  var root = document.documentElement;

  function stored() {
    try {
      return localStorage.getItem(KEY);
    } catch (e) {
      return null;
    }
  }

  function apply(theme) {
    if (theme === "light" || theme === "dark") {
      root.setAttribute("data-theme", theme);
    } else {
      root.removeAttribute("data-theme");
    }
  }

  // Pre-paint: honour a stored choice. Runs during head parse, before <body>.
  apply(stored());

  // The theme in force right now: an explicit choice wins, else the OS setting.
  function effective() {
    var attr = root.getAttribute("data-theme");
    if (attr === "light" || attr === "dark") return attr;
    return window.matchMedia &&
      window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }

  function relabel(btn) {
    var next = effective() === "dark" ? "light" : "dark";
    btn.setAttribute("aria-label", "Switch to " + next + " theme");
    btn.setAttribute("title", "Switch to " + next + " theme");
  }

  function init() {
    var btn = document.querySelector(".dock-theme");
    if (!btn) return;
    btn.hidden = false;
    relabel(btn);
    btn.addEventListener("click", function () {
      var next = effective() === "dark" ? "light" : "dark";
      apply(next);
      try {
        localStorage.setItem(KEY, next);
      } catch (e) {}
      relabel(btn);
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
