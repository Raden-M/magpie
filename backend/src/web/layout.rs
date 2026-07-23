//! Shared chrome for the human-facing UI pages (collection, search). The reader
//! has its own bespoke shell (it carries a rail and a different head).

use maud::{DOCTYPE, Markup, html};

/// The common page shell: `<head>` + top bar with the wordmark and search box.
pub fn shell(title: &str, search_value: &str, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) " · Magpie" }
                link rel="stylesheet" href="/assets/app.css";
            }
            body class="ui-body" {
                header class="topbar" {
                    a class="brand" href="/" { "Mag" span class="brand-pie" { "pie" } }
                    form class="search-form" method="get" action="/search" {
                        input class="search-input" type="search" name="q" value=(search_value)
                              placeholder="Search everything you kept";
                    }
                }
                (body)
            }
        }
    }
}
