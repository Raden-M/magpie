// The dock: two floating page controls, shared by every UI page.
//
// Progressive enhancement, as with the reader. "Back to top" is a plain `#top`
// anchor that already works on its own, so the script only upgrades it to a
// smooth scroll and keeps it out of the way until there is something to come
// back from. Refresh ships hidden and is revealed here: reloading the current
// URL is only reliable from script (an empty `href` next to a `#hl-…` fragment
// is a same-document navigation, and reloads nothing), and the browser already
// carries that control natively, so a broken-looking one is worse than none.
(function () {
  "use strict";

  var dock = document.querySelector(".dock");
  if (!dock) return;
  dock.setAttribute("data-js", "");

  var reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  var refresh = dock.querySelector(".dock-refresh");
  if (refresh) {
    refresh.hidden = false;
    refresh.addEventListener("click", function () {
      refresh.classList.add("is-busy");
      window.location.reload();
    });
  }

  var toTop = dock.querySelector(".dock-top");
  if (toTop) {
    toTop.addEventListener("click", function (event) {
      event.preventDefault();
      window.scrollTo({ top: 0, behavior: reduceMotion ? "auto" : "smooth" });
      // The link hides itself once the page is back at the top, so focus has to
      // land somewhere deliberate rather than falling back to <body>.
      var brand = document.querySelector(".topbar .brand");
      if (brand) brand.focus({ preventScroll: true });
    });
  }

  // The link earns its place after a screenful has gone by.
  var shown = false;
  var queued = false;

  function sync() {
    queued = false;
    var should = window.scrollY > window.innerHeight * 0.6;
    if (should === shown) return;
    shown = should;
    dock.classList.toggle("is-up", should);
  }

  window.addEventListener(
    "scroll",
    function () {
      if (queued) return;
      queued = true;
      window.requestAnimationFrame(sync);
    },
    { passive: true }
  );
  sync();
})();
