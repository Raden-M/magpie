// Live search over the collection.
//
// Progressive enhancement, like the dock and reader. Without this script the
// search box is a plain GET form: type a query, press Enter, and the server
// re-renders `/` with the matching rows. The script upgrades that to search as
// you type - it fetches the `/partials/collection` fragment (the same rows the
// full page would render) and swaps it into the list, so the markup has one
// source. It also drives the clear button and keeps the address bar in step, so
// "Load more", reload and the back button all carry the current query.
(function () {
  "use strict";

  var form = document.querySelector(".collection-search");
  var list = document.getElementById("collection-list");
  if (!form || !list) return;

  var input = form.querySelector(".collection-search-input");
  var clear = form.querySelector(".collection-search-clear");
  if (!input) return;

  form.setAttribute("data-js", "");
  // The fetch below owns updates now; a full-page submit would double the work
  // and throw away the box's focus.
  form.addEventListener("submit", function (event) {
    event.preventDefault();
    window.clearTimeout(timer);
    run();
  });

  var timer = null;
  // Each request carries a token; a slow response for an earlier keystroke must
  // not overwrite the list a later one already filled.
  var token = 0;

  function syncClear() {
    if (clear) clear.hidden = input.value.length === 0;
  }

  function run() {
    var q = input.value;
    var trimmed = q.trim();
    var mine = ++token;
    var url =
      trimmed === ""
        ? "/partials/collection"
        : "/partials/collection?q=" + encodeURIComponent(q);

    fetch(url, { credentials: "same-origin" })
      .then(function (res) {
        if (!res.ok) throw new Error(String(res.status));
        return res.text();
      })
      .then(function (html) {
        if (mine !== token) return;
        list.innerHTML = html;
      })
      .catch(function () {
        // Leave the last good list standing; the box still works.
      });

    var addr = trimmed === "" ? "/" : "/?q=" + encodeURIComponent(q);
    window.history.replaceState(null, "", addr);
  }

  input.addEventListener("input", function () {
    syncClear();
    window.clearTimeout(timer);
    timer = window.setTimeout(run, 150);
  });

  if (clear) {
    clear.addEventListener("click", function (event) {
      event.preventDefault();
      input.value = "";
      syncClear();
      input.focus();
      window.clearTimeout(timer);
      run();
    });
  }

  syncClear();
})();
