use gtk4 as gtk;
use libadwaita as adw;
use webkit6 as webkit;
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use std::cell::RefCell;
use async_openai::{
    config::OpenAIConfig,
    Client,
    types::{CreateChatCompletionRequestArgs, ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs},
};
use std::env;

use gtk::prelude::*;
use adw::prelude::*;
use webkit::prelude::*;
use futures::StreamExt;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AppSettings {
    provider: String,
    model: String,
    api_key: String,
    base_url: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            provider: "Nvidia".to_string(),
            model: "meta/llama-3.1-405b-instruct".to_string(),
            api_key: "".to_string(),
            base_url: "https://integrate.api.nvidia.com/v1".to_string(),
        }
    }
}

fn load_settings() -> AppSettings {
    if let Ok(data) = std::fs::read_to_string("atlas_settings.json") {
        if let Ok(settings) = serde_json::from_str(&data) {
            return settings;
        }
    }
    AppSettings::default()
}

fn save_settings(settings: &AppSettings) {
    if let Ok(data) = serde_json::to_string_pretty(settings) {
        let _ = std::fs::write("atlas_settings.json", data);
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PageContext {
    url: String,
    title: String,
    highlighted_text: String,
    main_content: String,
}

const NATIVE_HOMEPAGE: &str = r##"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Linux Atlas</title>
    <style>
        :root { --accent: #76B900; --bg: #121212; }
        body { margin: 0; padding: 0; display: flex; justify-content: center; align-items: center; height: 100vh; background: var(--bg); color: white; font-family: 'Inter', system-ui, sans-serif; overflow: hidden; }
        .container { text-align: center; width: 100%; max-width: 600px; animation: fadeIn 0.8s ease; }
        @keyframes fadeIn { from { opacity: 0; transform: translateY(15px); } to { opacity: 1; transform: translateY(0); } }
        .logo-wrap { position: relative; width: 140px; height: 140px; margin: 0 auto 20px; }
        .logo { width: 100%; height: 100%; filter: drop-shadow(0 0 25px rgba(118, 185, 0, 0.4)); animation: float 4s ease-in-out infinite; }
        @keyframes float { 0%, 100% { transform: translateY(0) rotate(0deg); } 50% { transform: translateY(-15px) rotate(2deg); } }
        h1 { font-size: 3.5rem; font-weight: 800; margin: 0; letter-spacing: -2px; background: linear-gradient(135deg, #fff 40%, var(--accent)); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
        .tagline { color: #666; font-size: 1rem; margin-bottom: 40px; text-transform: uppercase; letter-spacing: 3px; font-weight: 600; }
        .search-box { position: relative; background: #1e1e1e; border: 1px solid #333; border-radius: 18px; display: flex; padding: 5px; transition: all 0.3s; box-shadow: 0 10px 30px rgba(0,0,0,0.3); }
        .search-box:focus-within { border-color: var(--accent); box-shadow: 0 0 0 4px rgba(118, 185, 0, 0.1), 0 10px 30px rgba(0,0,0,0.5); transform: translateY(-2px); }
        input { flex: 1; background: transparent; border: none; padding: 15px 20px; font-size: 1.2rem; color: white; outline: none; }
        input::placeholder { color: #444; }
        .badge { background: rgba(118, 185, 0, 0.1); color: var(--accent); padding: 5px 15px; border-radius: 20px; font-size: 0.7rem; font-weight: 900; margin-bottom: 15px; display: inline-block; border: 1px solid rgba(118, 185, 0, 0.2); }
    </style>
</head>
<body>
    <div class="container">
        <div class="badge">LINUX ATLAS AI</div>
        <div class="logo-wrap">
            <svg class="logo" viewBox="0 0 100 100">
                <defs>
                    <linearGradient id="g" x1="0%" y1="0%" x2="100%" y2="100%">
                        <stop offset="0%" stop-color="#3584e4"/><stop offset="100%" stop-color="#76B900"/>
                    </linearGradient>
                </defs>
                <path d="M50 10 C35 10 25 30 25 50 C25 80 35 90 50 90 C65 90 75 80 75 50 C75 30 65 10 50 10 Z" fill="url(#g)"/>
                <path d="M50 35 C40 35 32 45 32 65 C32 80 40 85 50 85 C60 85 68 80 68 65 C68 45 60 35 50 35 Z" fill="#fff" opacity="0.9"/>
                <circle cx="43" cy="35" r="5" fill="#fff"/><circle cx="57" cy="35" r="5" fill="#fff"/>
                <circle cx="44" cy="35" r="2" fill="#121212"/><circle cx="56" cy="35" r="2" fill="#121212"/>
                <path d="M46 42 Q50 48 54 42" stroke="#FFA500" fill="none" stroke-width="2" stroke-linecap="round"/>
                <ellipse cx="35" cy="88" rx="8" ry="4" fill="#FFA500"/><ellipse cx="65" cy="88" rx="8" ry="4" fill="#FFA500"/>
            </svg>
        </div>
        <h1>Tux Search</h1>
        <div class="tagline">The Web, Without the Junk</div>
        <form id="f" class="search-box">
            <input type="text" id="i" placeholder="Search the web freely..." autofocus autocomplete="off">
        </form>
    </div>
    <script>
        window.onload = () => document.getElementById('i').focus();
        document.getElementById('f').onsubmit = (e) => {
            e.preventDefault();
            let v = document.getElementById('i').value.trim();
            if(!v) return;
            window.location.href = (v.includes('.') && !v.includes(' ')) ? (v.startsWith('http') ? v : 'https://'+v) : 'https://www.google.com/search?q='+encodeURIComponent(v);
        };
    </script>
</body>
</html>
"##;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    unsafe { std::env::set_var("G_TLS_GNUTLS_PRIORITY", "@SYSTEM:-VERS-TLS1.3"); }
    let app = adw::Application::builder().application_id("com.github.linux_atlas").build();
    app.connect_startup(|_| {
        let provider = gtk::CssProvider::new();
        provider.load_from_data(".sidebar-bg { background-color: @window_bg_color; border-left: 1px solid @borders; } .chat-bubble-user { background-color: @accent_bg_color; color: @accent_fg_color; border-radius: 14px; padding: 12px 16px; font-weight: 500; } .chat-bubble-ai { background-color: @card_bg_color; color: @card_fg_color; border-radius: 14px; padding: 12px 16px; border: 1px solid @borders; line-height: 1.5; }");
        gtk::style_context_add_provider_for_display(&gtk::gdk::Display::default().unwrap(), &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    });
    app.connect_activate(build_ui);
    app.run();
}

fn create_tab(tv: &adw::TabView, cm: &webkit::UserContentManager, st: &webkit::Settings, ah: Rc<RefCell<std::collections::HashSet<String>>>, ue: gtk::Entry, wc: &webkit::WebContext) -> webkit::WebView {
    let wv = webkit::WebView::builder().user_content_manager(cm).settings(st).web_context(wc).hexpand(true).vexpand(true).build();
    let tv_c = tv.clone(); let ue_c = ue.clone();
    wv.connect_uri_notify(move |w| { if let Some(p) = tv_c.selected_page() { if let Ok(cw) = p.child().downcast::<webkit::WebView>() { if cw == *w {
        if let Some(u) = w.uri() { ue_c.set_text(if u=="atlas://home" {""} else {&u}); }
    } } } });
    let tv_c2 = tv.clone();
    wv.connect_title_notify(move |w| { for i in 0..tv_c2.n_pages() { let p = tv_c2.nth_page(i); if let Ok(cw) = p.child().downcast::<webkit::WebView>() { if cw == *w {
        p.set_title(&w.title().unwrap_or_else(|| "New Tab".into())); break;
    } } } });
    let ah_c = ah.clone();
    wv.connect_decide_policy(move |w, d, dt| {
        if dt == webkit::PolicyDecisionType::NavigationAction { if let Some(nd) = d.downcast_ref::<webkit::NavigationPolicyDecision>() { if let Some(a) = nd.navigation_action() { if let Some(r) = a.request() { if let Some(u) = r.uri() {
            let us = u.as_str(); if us.starts_with("http://") && !us.starts_with("http://localhost") {
                let dom = us.split('/').nth(2).unwrap_or("").to_string();
                if !ah_c.borrow().contains(&dom) { d.ignore(); if let Some(win) = w.root().and_downcast::<gtk::Window>() {
                    let dlg = gtk::MessageDialog::builder().text("Unsafe Connection").secondary_text("HTTPS required. Proceed?").message_type(gtk::MessageType::Warning).buttons(gtk::ButtonsType::None).transient_for(&win).build();
                    dlg.add_button("No", gtk::ResponseType::Reject); dlg.add_button("Freedom", gtk::ResponseType::Accept);
                    let w_c = w.clone(); let u_c = us.to_string(); let ah_c2 = ah_c.clone();
                    let dom_c = dom.clone();
                    dlg.connect_response(move |dialog, res| { if res == gtk::ResponseType::Accept { ah_c2.borrow_mut().insert(dom_c.clone()); w_c.load_uri(&u_c); } dialog.destroy(); });
                    dlg.present();
                } return true; }
            }
        } } } } } false
    });
    wv.connect_load_failed(|_, _, _, e| { e.message().contains("close notify") || e.message().contains("fatal tls alert") });
    let page = tv.append(&wv); page.set_title("New Tab"); tv.set_selected_page(&page);
    wv
}

fn build_ui(app: &adw::Application) {
    let win = adw::ApplicationWindow::builder().application(app).title("Linux Atlas").default_width(1200).default_height(800).build();
    let app_settings = Rc::new(RefCell::new(load_settings()));
    let split = adw::Flap::builder().flap_position(gtk::PackType::End).fold_policy(adw::FlapFoldPolicy::Never).build();
    let cm = webkit::UserContentManager::new();
    cm.add_script(&webkit::UserScript::new("window.addEventListener('atlas:request_context', function() { let h = window.getSelection().toString(); let c = document.querySelector('article, main, [role=\"main\"]')?.innerText || document.body.innerText; let data = { url: window.location.href, title: document.title, highlighted_text: h, main_content: c.substring(0, 4000) }; window.webkit.messageHandlers.atlas_bridge.postMessage(JSON.stringify(data)); });", webkit::UserContentInjectedFrames::TopFrame, webkit::UserScriptInjectionTime::End, &[], &[]));
    cm.register_script_message_handler("atlas_bridge", None);
    let st = webkit::Settings::builder().user_agent("Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:124.0) Gecko/20100101 Firefox/124.0").enable_webaudio(true).enable_webgl(true).enable_media_stream(true).enable_developer_extras(true).build();
    let tv = adw::TabView::new(); let tb = adw::TabBar::builder().view(&tv).autohide(false).build();
    let ue = gtk::Entry::builder().placeholder_text("Search or enter address").hexpand(true).max_width_chars(50).build();
    let hb = adw::HeaderBar::new();
    let ntb = gtk::Button::from_icon_name("tab-new-symbolic"); let hom = gtk::Button::from_icon_name("go-home-symbolic"); let bck = gtk::Button::from_icon_name("go-previous-symbolic"); let fwd = gtk::Button::from_icon_name("go-next-symbolic"); let rel = gtk::Button::from_icon_name("view-refresh-symbolic"); let tai = gtk::Button::from_icon_name("view-sidebar-symbolic");
    hb.pack_start(&ntb); hb.pack_start(&hom); hb.pack_start(&bck); hb.pack_start(&fwd); hb.pack_start(&rel); hb.set_title_widget(Some(&ue)); hb.pack_end(&tai);
    let cache = format!("{}/.cache/atlas-browser", env::var("HOME").unwrap_or_else(|_| "/tmp".into())); let _ = std::fs::create_dir_all(&cache);
    let wc = webkit::WebContext::new();
    let ah = Rc::new(RefCell::new(std::collections::HashSet::new()));
    create_tab(&tv, &cm, &st, ah.clone(), ue.clone(), &wc).load_alternate_html(NATIVE_HOMEPAGE, "atlas://home", None);
    let tv_c = tv.clone(); let cm_c = cm.clone(); let st_c = st.clone(); let ue_c = ue.clone(); let ah_c = ah.clone(); let wc_c = wc.clone();
    ntb.connect_clicked(move |_| { create_tab(&tv_c, &cm_c, &st_c, ah_c.clone(), ue_c.clone(), &wc_c).load_alternate_html(NATIVE_HOMEPAGE, "atlas://home", None); });
    let tv_c = tv.clone(); hom.connect_clicked(move |_| { if let Some(p) = tv_c.selected_page() { if let Ok(w) = p.child().downcast::<webkit::WebView>() { w.load_alternate_html(NATIVE_HOMEPAGE, "atlas://home", None); } } });
    let tv_c = tv.clone(); bck.connect_clicked(move |_| { if let Some(p) = tv_c.selected_page() { if let Ok(w) = p.child().downcast::<webkit::WebView>() { w.go_back(); } } });
    let tv_c = tv.clone(); fwd.connect_clicked(move |_| { if let Some(p) = tv_c.selected_page() { if let Ok(w) = p.child().downcast::<webkit::WebView>() { w.go_forward(); } } });
    let tv_c = tv.clone(); rel.connect_clicked(move |_| { if let Some(p) = tv_c.selected_page() { if let Ok(w) = p.child().downcast::<webkit::WebView>() { w.reload(); } } });
    let tv_c = tv.clone(); ue.connect_activate(move |e| { let v = e.text().to_string(); let uri = if v.starts_with("http") { v } else if v.contains('.') && !v.contains(' ') { format!("https://{}", v) } else { format!("https://www.google.com/search?q={}", v) }; if let Some(p) = tv_c.selected_page() { if let Ok(w) = p.child().downcast::<webkit::WebView>() { w.load_uri(&uri); } } });
    let ue_c = ue.clone(); tv.connect_selected_page_notify(move |t| { if let Some(p) = t.selected_page() { if let Ok(w) = p.child().downcast::<webkit::WebView>() { ue_c.set_text(&w.uri().unwrap_or_else(|| "".into()).replace("atlas://home", "")); } } });
    tv.connect_close_page(move |t, p| { t.close_page_finish(p, true); true.into() });
    let mv = gtk::Box::new(gtk::Orientation::Vertical, 0); mv.append(&hb); mv.append(&tb); mv.append(&tv);
    let sb = gtk::Box::builder().orientation(gtk::Orientation::Vertical).width_request(400).css_classes(["sidebar-bg"]).build();
    let ch = adw::HeaderBar::new(); let tl = gtk::Label::new(Some("Atlas AI")); tl.add_css_class("title"); ch.set_title_widget(Some(&tl));
    let stb = gtk::Button::from_icon_name("emblem-system-symbolic"); ch.pack_end(&stb);
    let cs = gtk::ScrolledWindow::builder().vexpand(true).build(); let cb = gtk::Box::new(gtk::Orientation::Vertical, 16); cb.set_margin_start(16); cb.set_margin_end(16); cb.set_margin_top(16); cb.set_margin_bottom(16); cs.set_child(Some(&cb));
    let ci = gtk::Entry::builder().placeholder_text("Ask Atlas AI...").margin_start(12).margin_end(12).margin_top(12).margin_bottom(12).build();
    sb.append(&ch); sb.append(&cs); sb.append(&ci); split.set_content(Some(&mv)); split.set_flap(Some(&sb)); split.set_reveal_flap(false);

    let as_c = app_settings.clone(); let wn_c = win.clone();
    stb.connect_clicked(move |_| {
        let pw = adw::Window::builder().title("Settings").default_width(450).default_height(500).modal(true).transient_for(&wn_c).build();
        let bx = gtk::Box::new(gtk::Orientation::Vertical, 0); let h = adw::HeaderBar::new(); bx.append(&h);
        let pg = adw::PreferencesPage::new(); let gr = adw::PreferencesGroup::new(); gr.set_title("API Config");
        let pr = adw::ComboRow::builder().title("Provider").build(); let pl = gtk::StringList::new(&["Nvidia", "Gemini", "OpenAI", "Custom"]); pr.set_model(Some(&pl));
        let mr = adw::ComboRow::builder().title("Model").build();
        let ar = adw::ActionRow::builder().title("API Key").build();
        let ke = gtk::PasswordEntry::builder().hexpand(true).valign(gtk::Align::Center).show_peek_icon(true).build(); ar.add_suffix(&ke);
        let br = adw::ActionRow::builder().title("Base URL").build();
        let be = gtk::Entry::builder().hexpand(true).valign(gtk::Align::Center).build(); br.add_suffix(&be);
        let cur = as_c.borrow().clone(); pr.set_selected(match cur.provider.as_str() {"Gemini"=>1,"OpenAI"=>2,"Custom"=>3,_=>0});
        ke.set_text(&cur.api_key); be.set_text(&cur.base_url); let ml = gtk::StringList::new(&[cur.model.as_str()]); mr.set_model(Some(&ml));
        gr.add(&pr); gr.add(&mr); gr.add(&ar); gr.add(&br); pg.add(&gr); bx.append(&pg);
        let sn = gtk::Button::builder().label("Sync Models").css_classes(["suggested-action"]).margin_start(16).margin_end(16).margin_top(8).build();
        let ap = gtk::Button::builder().label("Apply").css_classes(["suggested-action"]).margin_start(16).margin_end(16).margin_top(8).margin_bottom(16).build();
        bx.append(&sn); bx.append(&ap); pw.set_content(Some(&bx));
        let mr_c = mr.clone(); let pr_c = pr.clone(); let ke_c = ke.clone(); let be_c = be.clone(); let sn_c = sn.clone();
        sn.connect_clicked(move |_| {
            let prov = match pr_c.selected() {1=>"Gemini",2=>"OpenAI",3=>"Custom",_=>"Nvidia"}.to_string();
            let key = ke_c.text().to_string(); let url = be_c.text().to_string();
            let b = sn_c.clone(); let m = mr_c.clone(); b.set_label("Syncing..."); b.set_sensitive(false);
            gtk::glib::spawn_future_local(async move {
                let mut cfg = OpenAIConfig::new().with_api_key(&key).with_api_base(&url);
                if prov == "Nvidia" { cfg = cfg.with_api_base("https://integrate.api.nvidia.com/v1"); }
                else if prov == "Gemini" { cfg = cfg.with_api_base("https://generativelanguage.googleapis.com/v1beta/openai/"); }
                let client = Client::with_config(cfg);
                if let Ok(r) = client.models().list().await {
                    let mut ms: Vec<String> = r.data.iter().map(|m| m.id.clone()).filter(|id| !id.contains("vision") && !id.contains("embed")).collect(); ms.sort();
                    let nl = gtk::StringList::new(&ms.iter().map(|s| s.as_str()).collect::<Vec<&str>>()); m.set_model(Some(&nl)); b.set_label("Sync Complete");
                } else { b.set_label("Sync Failed"); }
                b.set_sensitive(true);
            });
        });
        let as_s = as_c.clone(); let pw_s = pw.clone();
        ap.connect_clicked(move |_| {
            let mut s = as_s.borrow_mut(); s.provider = match pr.selected() {1=>"Gemini".into(),2=>"OpenAI".into(),3=>"Custom".into(),_=>"Nvidia".into()};
            if let Some(i) = mr.selected_item() { if let Ok(so) = i.downcast::<gtk::StringObject>() { s.model = so.string().to_string(); } }
            s.api_key = ke.text().to_string(); s.base_url = be.text().to_string(); save_settings(&s); pw_s.destroy();
        });
        pw.present();
    });

    let l_ctx: Rc<RefCell<Option<PageContext>>> = Rc::new(RefCell::new(None));
    let lc_c = l_ctx.clone();
    cm.connect_script_message_received(Some("atlas_bridge"), move |_, m| {
        if let Some(jv) = m.to_json(0) { if let Ok(us) = serde_json::from_str::<String>(&jv.to_string()) {
            if let Ok(ctx) = serde_json::from_str::<PageContext>(&us) { *lc_c.borrow_mut() = Some(ctx); }
        } }
    });

    let sv_c = split.clone(); let tv_c = tv.clone();
    tai.connect_clicked(move |_| {
        sv_c.set_reveal_flap(!sv_c.reveals_flap());
        if sv_c.reveals_flap() { if let Some(p) = tv_c.selected_page() { if let Ok(w) = p.child().downcast::<webkit::WebView>() {
            w.evaluate_javascript("window.dispatchEvent(new Event('atlas:request_context'));", None, None, None::<&gtk::gio::Cancellable>, |_| {});
        } } }
    });
    
    let cb_c = cb.clone(); let lc_ai = l_ctx.clone(); let cs_c = cs.clone(); let tv_ai = tv.clone(); let as_ai = app_settings.clone();
    ci.connect_activate(move |entry| {
        let pmt = entry.text().to_string(); if pmt.is_empty() { return; }
        entry.set_text("");
        let u_w = gtk::Box::new(gtk::Orientation::Horizontal, 0); u_w.set_halign(gtk::Align::End);
        let u_l = gtk::Label::builder().label(&pmt).wrap(true).css_classes(["chat-bubble-user"]).build();
        u_w.append(&u_l); cb_c.append(&u_w);
        let a_w = gtk::Box::new(gtk::Orientation::Horizontal, 0); a_w.set_halign(gtk::Align::Start);
        let a_l = gtk::Label::builder().label("Thinking...").wrap(true).css_classes(["chat-bubble-ai"]).build();
        a_w.append(&a_l); cb_c.append(&a_w);
        let adj = cs_c.vadjustment(); adj.set_value(adj.upper());
        let ctx = lc_ai.borrow().clone(); let (tx, rx) = async_channel::unbounded::<String>();
        let al_c = a_l.clone(); let tv_c2 = tv_ai.clone(); let chs_c = cs_c.clone();
        gtk::glib::spawn_future_local(async move {
            let mut fl = String::new();
            while let Ok(ck) = rx.recv().await {
                if ck == "[DONE]" {
                    if fl.contains("[CLICK: ") {
                        let s = fl.find("[CLICK: ").unwrap()+8; let e = fl[s..].find(']').unwrap()+s;
                        let sel = &fl[s..e];
                        let script = format!("(function(){{let t=document.querySelector('{}');if(!t)return;let r=t.getBoundingClientRect();let x=r.left+r.width/2,y=r.top+r.height/2;let c=document.getElementById('at-c');if(!c){{c=document.createElement('div');c.id='at-c';c.innerHTML='<svg width=\"24\" height=\"24\" viewBox=\"0 0 24 24\"><path d=\"M5.5 3.21V20.8c0 .45.54.67.85.35l4.86-4.86a.5.5 0 01.35-.15h6.94c.45 0 .67-.54.35-.85L6.35 2.86a.5.5 0 00-.85.35z\" fill=\"#76B900\" stroke=\"white\" stroke-width=\"1.5\"/></svg>';c.style='position:fixed;z-index:999999;pointer-events:none;transition:all 0.6s cubic-bezier(0.2,0.8,0.2,1);filter:drop-shadow(0 0 10px rgba(0,0,0,0.5))';document.body.appendChild(c);}}c.style.left=x+'px';c.style.top=y+'px';setTimeout(()=>{{t.click();}},700);}})();", sel);
                        if let Some(p) = tv_c2.selected_page() { if let Ok(w) = p.child().downcast::<webkit::WebView>() { w.evaluate_javascript(&script,None,None,None::<&gtk::gio::Cancellable>,|_|{}); } }
                    } break;
                }
                fl.push_str(&ck); let tx_val = al_c.text().to_string(); let new_label = if tx_val=="Thinking..." { ck } else { tx_val + &ck }; al_c.set_label(&new_label);
                let a = chs_c.vadjustment(); a.set_value(a.upper());
            }
        });
        let tx_c = tx.clone(); let set = as_ai.borrow().clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if set.api_key.is_empty() { let _=tx_c.send("Set API Key in ⚙️.".into()).await; let _=tx_c.send("[DONE]".into()).await; return; }
                let mut cfg = OpenAIConfig::new().with_api_key(&set.api_key).with_api_base(&set.base_url);
                if set.provider=="Nvidia"{cfg=cfg.with_api_base("https://integrate.api.nvidia.com/v1");}
                else if set.provider=="Gemini"{cfg=cfg.with_api_base("https://generativelanguage.googleapis.com/v1beta/openai/");}
                let client = Client::with_config(cfg);
                let mut sys = String::from("You are Atlas AI. If asked to click, respond with ONLY [CLICK: selector]. ");
                if let Some(c) = ctx { if !c.url.contains("atlas://") { sys.push_str(&format!("URL: {}. Page: {}. Context: {}", c.url, c.title, c.main_content)); } }
                let req = CreateChatCompletionRequestArgs::default().model(&set.model).messages([ChatCompletionRequestSystemMessageArgs::default().content(sys).build().unwrap().into(), ChatCompletionRequestUserMessageArgs::default().content(pmt).build().unwrap().into()]).build().unwrap();
                match client.chat().create_stream(req).await {
                    Ok(mut s) => { while let Some(Ok(r)) = s.next().await { for ch in r.choices { if let Some(con) = ch.delta.content { let _=tx_c.send(con).await; } } } let _=tx_c.send("[DONE]".into()).await; }
                    Err(e) => { let _=tx_c.send(format!("[ERROR] {:?}", e)).await; }
                }
            });
        });
    });
    win.set_content(Some(&split)); win.present();
}
