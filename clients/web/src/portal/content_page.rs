use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use pulldown_cmark::{Options, Parser, html};

use crate::api;
use crate::i18n::*;

#[component]
pub fn ContentPage() -> impl IntoView {
    let params = use_params_map();
    let slug = move || params.read().get("slug").unwrap_or_default();
    let i18n = use_i18n();

    let page = LocalResource::new(move || {
        let s = slug();
        let lang = match i18n.get_locale() {
            Locale::en => "en",
            Locale::fr => "fr",
        };
        async move { api::get_page(&s, lang).await }
    });

    view! {
        <div class="portal-main">
            {move || match page.get().map(|sw| sw.take()) {
                None => view! {
                    <p class="portal-loading">{t!(i18n, loading)}</p>
                }.into_any(),
                Some(Err(_)) => view! {
                    <p class="portal-empty">"Page not found."</p>
                }.into_any(),
                Some(Ok(p)) => {
                    let html = md_to_html(&p.content);
                    view! {
                        <div class="portal-card content-page" inner_html=html />
                    }.into_any()
                }
            }}
        </div>
    }
}

fn md_to_html(md: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(md, opts);
    let mut output = String::new();
    html::push_html(&mut output, parser);
    output
}
