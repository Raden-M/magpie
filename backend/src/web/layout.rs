//! Shared chrome for the human-facing UI pages (collection, search). The reader
//! has its own bespoke shell (it carries a rail and a different head).

use maud::{DOCTYPE, Markup, PreEscaped, html};

/// Inlined rather than linked so the mark can read the theme: its silhouette is
/// `--ink` and its breast `--ground`, so the bird inverts between light and dark
/// while the blue shine on wing and tail stays put. An `<img>` would isolate it
/// from our custom properties and freeze one colouring.
///
/// It shares artwork with `extension/src/icon/magpie.svg`, which bakes the
/// colours in and adds a tile - a browser toolbar icon can read neither our CSS
/// nor the page behind it. The two are edited together; Docker builds the
/// backend with `./backend` as its context, so they can't be one file.
const MAGPIE_MARK: &str = include_str!("../assets/magpie-mark.svg");

/// The brand lockup: the mark beside the wordmark, with the slogan set beneath in
/// the reading italic - so the promise is written in the product's own reading
/// voice. Shared by every shell that shows it.
pub fn brand() -> Markup {
    html! {
        a class="brand" href="/" aria-label="Magpie" {
            (PreEscaped(MAGPIE_MARK))
            span class="brand-stack" {
                span class="brand-word" { "Mag" span class="brand-pie" { "pie" } }
                span class="brand-slogan" {
                    span class="brand-slogan-lead" { "In search of" }
                    " shiny things"
                }
            }
        }
    }
}

const ICON_BACK: &str = r##"<svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M15 18l-6-6 6-6"/></svg>"##;

/// A labeled return to the collection, for the chrome of pages that are not the
/// collection itself (the reader, the about page). The wordmark links home too,
/// but a page deep in an item wants an obvious way back, not just a clickable
/// logo - and a reader opened cold (from a capture toast, a bookmark) has no
/// history to go back through.
pub fn back_to_collection() -> Markup {
    html! {
        a class="topbar-back" href="/" {
            (PreEscaped(ICON_BACK))
            span { "Collection" }
        }
    }
}

/// Two line drawings on a 24 grid. Inlined for the same reason the mark is: they
/// take `currentColor`, so they follow the button's own hover and focus states,
/// which an `<img>` could not. Two glyphs don't justify an icon dependency.
const ICON_TOP: &str = r##"<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M12 19V6"/><path d="M6 12l6-6 6 6"/></svg>"##;
const ICON_REFRESH: &str = r##"<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="M21 12a9 9 0 1 1-2.64-6.36L21 8"/><path d="M21 3v5h-5"/></svg>"##;
/// Sun and moon for the theme toggle, both painted; CSS shows the one for the
/// theme in force (see `.dock-theme` in `app.css`) - the sun on the dark theme,
/// the moon on the light. The icon names the theme you would switch *to*.
const ICON_SUN: &str = r##"<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" class="icon-sun" aria-hidden="true" focusable="false"><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/></svg>"##;
const ICON_MOON: &str = r##"<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" class="icon-moon" aria-hidden="true" focusable="false"><path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8Z"/></svg>"##;

/// The dock: theme, refresh and back-to-top, floating bottom-right on every UI
/// page. Refresh matters because captures land while a page is open - the
/// extension writes to the backend behind whatever you are reading.
///
/// The buttons are progressive, and differently so (see `assets/dock.js` and
/// `assets/theme.js`): the anchor works without script and the script only
/// refines it, while the two buttons ship `hidden` because a control that cannot
/// act (a reload, a theme swap) is worse than a dead one.
pub fn dock() -> Markup {
    html! {
        div class="dock" {
            a class="dock-btn dock-top" href="#top" aria-label="Back to top" title="Back to top" {
                (PreEscaped(ICON_TOP))
            }
            button type="button" class="dock-btn dock-refresh" hidden
                   aria-label="Refresh" title="Refresh" {
                (PreEscaped(ICON_REFRESH))
            }
            button type="button" class="dock-btn dock-theme" hidden
                   aria-label="Switch theme" title="Switch theme" {
                (PreEscaped(ICON_SUN))
                (PreEscaped(ICON_MOON))
            }
        }
    }
}

/// The common page shell: `<head>` + top bar with the wordmark. Search lives in
/// the collection body now (a live box above the list), not the chrome.
pub fn shell(title: &str, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " · Magpie" }
                // Ahead of the stylesheet so a stored theme is stamped on <html>
                // before the CSS resolves - no flash of the wrong theme.
                script src=(super::assets::theme_js_url()) {}
                link rel="stylesheet" href=(super::assets::app_css_url());
            }
            body class="ui-body" {
                // `#top` is spec'd to mean the top of the document with no such
                // element, but naming the bar makes the dock's anchor explicit.
                header id="top" class="topbar" {
                    (brand())
                }
                (body)
                (dock())
                script src=(super::assets::dock_js_url()) {}
                script src=(super::assets::collection_js_url()) defer {}
            }
        }
    }
}
