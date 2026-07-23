// Reader scroll-spy island. Progressive enhancement only: the reader is fully
// functional without JS (baked <mark> spans, anchor-link jump-to). This adds a
// soft glow on the highlight you are reading and syncs the rail selection.
(function () {
  "use strict";

  var marks = Array.prototype.slice.call(document.querySelectorAll(".article [data-hl]"));
  if (marks.length === 0) return;

  var railItems = {};
  document.querySelectorAll(".rail-item[data-hl]").forEach(function (item) {
    railItems[item.getAttribute("data-hl")] = item;
  });

  var reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  var activeId = null;

  function setActive(id) {
    if (id === activeId) return;
    if (activeId) {
      document
        .querySelectorAll('[data-hl="' + cssEscape(activeId) + '"]')
        .forEach(function (el) { el.classList.remove("is-active"); });
    }
    activeId = id;
    if (id) {
      document
        .querySelectorAll('[data-hl="' + cssEscape(id) + '"]')
        .forEach(function (el) { el.classList.add("is-active"); });
    }
  }

  function cssEscape(value) {
    if (window.CSS && window.CSS.escape) return window.CSS.escape(value);
    return value.replace(/["\\\]]/g, "\\$&");
  }

  // Track which highlighted marks are on screen; the topmost one is "active".
  var visible = new Set();
  var observer = new IntersectionObserver(
    function (entries) {
      entries.forEach(function (entry) {
        var id = entry.target.getAttribute("data-hl");
        if (entry.isIntersecting) visible.add(id);
        else visible.delete(id);
      });
      var topmost = null;
      var topY = Infinity;
      marks.forEach(function (mark) {
        var id = mark.getAttribute("data-hl");
        if (!visible.has(id)) return;
        var y = mark.getBoundingClientRect().top;
        if (y < topY) { topY = y; topmost = id; }
      });
      setActive(topmost);
    },
    { rootMargin: "-10% 0px -70% 0px", threshold: 0 }
  );
  marks.forEach(function (mark) { observer.observe(mark); });

  // Smooth scroll + activation when a rail item is clicked.
  Object.keys(railItems).forEach(function (id) {
    var link = railItems[id].querySelector(".rail-quote");
    if (!link || link.tagName !== "A") return;
    link.addEventListener("click", function (event) {
      var target = document.getElementById("hl-" + id);
      if (!target) return;
      event.preventDefault();
      target.scrollIntoView({
        behavior: reduceMotion ? "auto" : "smooth",
        block: "center",
      });
      setActive(id);
      history.replaceState(null, "", "#hl-" + id);
    });
  });

  // Reflect the active mark in the rail.
  var railObserver = new MutationObserver(syncRail);
  function syncRail() {
    Object.keys(railItems).forEach(function (id) {
      railItems[id].classList.toggle("is-active", id === activeId);
    });
  }
  railObserver.observe(document.body, { attributes: true, subtree: true, attributeFilter: ["class"] });
  window.addEventListener("scroll", syncRail, { passive: true });
})();
