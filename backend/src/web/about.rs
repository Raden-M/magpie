//! `GET /about` - what Magpie is. A short editorial piece on the idea behind the
//! app: structural colour, and keeping the things that catch the light. It reads
//! on the same reading surface as an article, in the same two editorial faces,
//! so the page argues its point by being an example of it.

use axum::http::header;
use axum::response::{Html, IntoResponse, Response};
use maud::{DOCTYPE, Markup, html};

use crate::web::PAGE_CSP;
use crate::web::layout;

pub async fn page() -> Response {
    (
        [(header::CONTENT_SECURITY_POLICY, PAGE_CSP)],
        Html(render().into_string()),
    )
        .into_response()
}

fn render() -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "What Magpie is · Magpie" }
                // Ahead of the stylesheet so a stored theme lands before paint.
                script src=(super::assets::theme_js_url()) {}
                link rel="stylesheet" href=(super::assets::app_css_url());
            }
            body class="reader-body" {
                header id="top" class="topbar" {
                    (layout::brand())
                    (layout::back_to_collection())
                }
                main class="about" {
                    p class="about-eyebrow" { "The idea" }
                    h1 class="about-title" { "A feather holds no blue." }
                    p class="about-lede" {
                        "The colour is an interference of light - a property of structure, not "
                        "pigment. Grind the feather to dust and the blue is gone. It was never in "
                        "the material; it was in the way the light was caught."
                    }

                    div class="about-body" {
                        p { "Everything Magpie believes is in that sentence." }

                        h2 { "Magpies keep what catches the light" }
                        p {
                            "Magpies are collectors by temperament. They notice the bright thing "
                            "in the hedge and carry it home - not to hoard for its own sake, but "
                            "to have it, to turn it over, to come back to it."
                        }
                        p {
                            "Magpie does the same with what you read. When a sentence catches you, "
                            "you keep it - a "
                            em { "shiny thing" }
                            " - and Magpie saves the page around it, so the catch survives the "
                            "internet's habit of quietly deleting itself. Your highlights are the "
                            "point; the snapshot is only the setting that keeps them safe."
                        }

                        h2 { "In search of shiny things" }
                        p {
                            "A hoard is worth having only if you can find your way back into it. "
                            "So everything you keep is searchable, re-readable, and a click from "
                            "where you left it. The shine is the product; the reader, the rail, "
                            "and the quiet monochrome around them exist only to let it show."
                        }

                        p class="about-close" { "Keep what catches the light." }
                    }
                }
                (layout::dock())
                script src=(super::assets::dock_js_url()) {}
            }
        }
    }
}
