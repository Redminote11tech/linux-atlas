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
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            provider: "Nvidia".to_string(),
            model: "meta/llama-3.1-405b-instruct".to_string(),
            api_key: "".to_string(),
        }
    }
}

fn load_settings() -> AppSettings {
    if let Ok(data) = std::fs::read_to_string("atlas_settings.json") {
        if let Ok(settings) = serde_json::from_str(&data) {
            return settings;
        }
    }
    let mut default = AppSettings::default();
    if let Ok(env_key) = env::var("NVIDIA_API_KEY") {
        if !env_key.starts_with("nvapi-XXXX") {
            default.api_key = env_key;
        }
    }
    default
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
        body {
            margin: 0;
            padding: 0;
            display: flex;
            flex-direction: column;
            justify-content: center;
            align-items: center;
            height: 100vh;
            background-color: #1e1e1e;
            color: #ffffff;
            font-family: system-ui, -apple-system, sans-serif;
            overflow: hidden;
        }
        .container {
            text-align: center;
            animation: fadein 0.8s ease-out;
        }
        @keyframes fadein {
            from { opacity: 0; transform: scale(0.95); }
            to { opacity: 1; transform: scale(1); }
        }
        .logo {
            width: 120px;
            height: 120px;
            margin: 0 auto 24px auto;
            filter: drop-shadow(0 0 20px rgba(118, 185, 0, 0.5));
            animation: float 3s ease-in-out infinite;
        }
        @keyframes float {
            0%, 100% { transform: translateY(0); }
            50% { transform: translateY(-15px); }
        }
        h1 {
            font-size: 3rem;
            font-weight: 800;
            margin-bottom: 4px;
            letter-spacing: -1px;
            background: linear-gradient(135deg, #ffffff 0%, #a0a0a0 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }
        p {
            font-size: 1.2rem;
            color: #76B900;
            margin-bottom: 40px;
            font-weight: 500;
            opacity: 0.9;
        }
        .search-box {
            display: flex;
            width: 100%;
            width: 650px;
            background: rgba(255, 255, 255, 0.05);
            border-radius: 30px;
            padding: 6px 10px;
            backdrop-filter: blur(10px);
            border: 1px solid rgba(255, 255, 255, 0.1);
            transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
        }
        .search-box:focus-within {
            background: rgba(255, 255, 255, 0.08);
            border-color: #76B900;
            box-shadow: 0 0 30px rgba(118, 185, 0, 0.2);
            transform: scale(1.02);
        }
        input {
            flex-grow: 1;
            background: transparent;
            border: none;
            padding: 14px 20px;
            font-size: 1.2rem;
            color: white;
            outline: none;
        }
        input::placeholder {
            color: rgba(255, 255, 255, 0.3);
        }
        .badge {
            background: #3584e4;
            color: white;
            padding: 4px 12px;
            border-radius: 20px;
            font-size: 0.8rem;
            margin-bottom: 20px;
            display: inline-block;
            font-weight: bold;
            text-transform: uppercase;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="badge">Experimental AI Browser</div>
        <svg class="logo" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
            <defs>
                <linearGradient id="tuxGrad" x1="0%" y1="0%" x2="100%" y2="100%">
                    <stop offset="0%" stop-color="#3584e4" />
                    <stop offset="100%" stop-color="#76B900" />
                </linearGradient>
                <linearGradient id="tuxBelly" x1="0%" y1="0%" x2="0%" y2="100%">
                    <stop offset="0%" stop-color="#e0e0e0" />
                    <stop offset="100%" stop-color="#ffffff" />
                </linearGradient>
            </defs>
            <path d="M50 10 C35 10 25 30 25 50 C25 80 35 90 50 90 C65 90 75 80 75 50 C75 30 65 10 50 10 Z" fill="url(#tuxGrad)"/>
            <path d="M50 35 C40 35 32 45 32 65 C32 80 40 85 50 85 C60 85 68 80 68 65 C68 45 60 35 50 35 Z" fill="url(#tuxBelly)"/>
            <circle cx="43" cy="35" r="5" fill="#fff"/><circle cx="57" cy="35" r="5" fill="#fff"/>
            <circle cx="44" cy="35" r="2" fill="#242424"/><circle cx="56" cy="35" r="2" fill="#242424"/>
            <path d="M46 42 Q50 48 54 42 Q50 44 46 42 Z" fill="#FFA500"/>
            <path d="M46 42 Q50 46 54 42 Q50 40 46 42 Z" fill="#FF8C00"/>
            <ellipse cx="35" cy="88" rx="8" ry="4" fill="#FFA500"/>
            <ellipse cx="65" cy="88" rx="8" ry="4" fill="#FFA500"/>
        </svg>
        <h1>Tux Search</h1>
        <p>Your Private Linux Portal</p>
        <form class="search-box" id="search-form">
            <input type="text" id="search-input" placeholder="Search with DuckDuckGo or enter URL..." autofocus autocomplete="off">
        </form>
    </div>
    <script>
        document.getElementById('search-form').addEventListener('submit', function(e) {
            e.preventDefault();
            let query = document.getElementById('search-input').value.trim();
            if (!query) return;
            
            let uri = query;
            if (query.startsWith('http://') || query.startsWith('https://')) {
            } else if (query.includes('.') && !query.includes(' ')) {
                uri = 'https://' + query;
            } else {
                uri = 'https://duckduckgo.com/?q=' + encodeURIComponent(query);
            }
            window.location.href = uri;
        });
    </script>
</body>
</html>
"##;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    
    // Nuclear fix for Fedora TLS bugs
    unsafe { std::env::set_var("G_TLS_GNUTLS_PRIORITY", "@SYSTEM:-VERS-TLS1.3"); }
    
    let app = adw::Application::builder()
        .application_id("com.github.linux_atlas")
        .build();

    app.connect_startup(|_| {
        let provider = gtk::CssProvider::new();
        provider.load_from_data(
            "
            .sidebar-bg {
                background-color: @window_bg_color;
                border-left: 1px solid @borders;
            }
            .chat-bubble-user {
                background-color: @accent_bg_color;
                color: @accent_fg_color;
                border-radius: 12px;
                padding: 10px 14px;
            }
            .chat-bubble-ai {
                background-color: @card_bg_color;
                color: @card_fg_color;
                border-radius: 12px;
                padding: 10px 14px;
                border: 1px solid @borders;
            }
            "
        );
        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    app.connect_activate(build_ui);

    app.run();
}

fn create_tab(
    tab_view: &adw::TabView,
    content_manager: &webkit::UserContentManager,
    settings: &webkit::Settings,
    accepted_http: Rc<RefCell<std::collections::HashSet<String>>>,
    url_entry: gtk::Entry,
) -> webkit::WebView {
    let web_view = webkit::WebView::builder()
        .user_content_manager(content_manager)
        .settings(settings)
        .hexpand(true)
        .vexpand(true)
        .build();

    let tv_clone = tab_view.clone();
    let ue_clone = url_entry.clone();
    web_view.connect_uri_notify(move |wv| {
        if let Some(page) = tv_clone.selected_page() {
            if let Ok(child_wv) = page.child().downcast::<webkit::WebView>() {
                if child_wv == *wv {
                    if let Some(uri) = wv.uri() {
                        if uri == "atlas://home" || uri == "atlas://home/" {
                            ue_clone.set_text("");
                        } else {
                            ue_clone.set_text(&uri);
                        }
                    }
                }
            }
        }
    });

    let tv_clone2 = tab_view.clone();
    web_view.connect_title_notify(move |wv| {
        for i in 0..tv_clone2.n_pages() {
            let page = tv_clone2.nth_page(i);
            if let Ok(child_wv) = page.child().downcast::<webkit::WebView>() {
                if child_wv == *wv {
                    if let Some(title) = wv.title() {
                        if title.is_empty() {
                            page.set_title("New Tab");
                        } else {
                            page.set_title(&title);
                        }
                    }
                    break;
                }
            }
        }
    });

    let ah_clone = accepted_http.clone();
    web_view.connect_decide_policy(move |wv, decision, decision_type| {
        if decision_type == webkit::PolicyDecisionType::NavigationAction {
            if let Some(nav_decision) = decision.downcast_ref::<webkit::NavigationPolicyDecision>() {
                if let Some(action) = nav_decision.navigation_action() {
                    if let Some(request) = action.request() {
                        if let Some(uri) = request.uri() {
                            let uri_str = uri.as_str();
                            if uri_str.starts_with("http://") && !uri_str.starts_with("http://localhost") && !uri_str.starts_with("http://127.0.0.1") {
                                let domain = uri_str.split('/').nth(2).unwrap_or("").to_string();
                                if !ah_clone.borrow().contains(&domain) {
                                    decision.ignore();
                                    
                                    if let Some(win) = wv.root().and_downcast::<gtk::Window>() {
                                        let dialog = gtk::MessageDialog::builder()
                                            .text("Unsafe Connection")
                                            .secondary_text("This website is not using HTTPS (TLS). Your connection is not private.\n\nAre you sure you want to proceed?")
                                            .message_type(gtk::MessageType::Warning)
                                            .buttons(gtk::ButtonsType::None)
                                            .transient_for(&win)
                                            .build();
                                            
                                        dialog.add_button("uhh no, with linux", gtk::ResponseType::Reject);
                                        dialog.add_button("Yes, I have Freedom", gtk::ResponseType::Accept);
                                        
                                        let wv_c = wv.clone();
                                        let uri_c = uri_str.to_string();
                                        let ah_c2 = ah_clone.clone();
                                        
                                        dialog.connect_response(move |dlg, response| {
                                            if response == gtk::ResponseType::Accept {
                                                ah_c2.borrow_mut().insert(domain.clone());
                                                wv_c.load_uri(&uri_c);
                                            }
                                            dlg.destroy();
                                        });
                                        dialog.present();
                                    }
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    });

    web_view.connect_load_failed(|_, _load_event, _uri, error| {
        let err_msg = error.message();
        if err_msg.contains("peer sent fatal tls alert") || err_msg.contains("close notify") {
            return true;
        }
        false 
    });

    let page = tab_view.append(&web_view);
    page.set_title("New Tab");
    tab_view.set_selected_page(&page);
    
    web_view
}

fn build_ui(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Linux Atlas")
        .default_width(1200)
        .default_height(800)
        .build();

    let app_settings = Rc::new(RefCell::new(load_settings()));

    let split_view = adw::Flap::builder()
        .flap_position(gtk::PackType::End)
        .fold_policy(adw::FlapFoldPolicy::Never)
        .build();

    let content_manager = webkit::UserContentManager::new();

    let extraction_script = webkit::UserScript::new(
        r#"
        window.addEventListener('atlas:request_context', function(e) {
            let highlighted = window.getSelection().toString();
            
            let mainContent = "";
            let article = document.querySelector('article, main, [role="main"]');
            if (article) {
                mainContent = article.innerText;
            } else {
                mainContent = document.body.innerText;
            }
            
            mainContent = mainContent.substring(0, 4000);

            let contextData = {
                url: window.location.href,
                title: document.title,
                highlighted_text: highlighted,
                main_content: mainContent
            };
            
            window.webkit.messageHandlers.atlas_bridge.postMessage(JSON.stringify(contextData));
        });
        "#,
        webkit::UserContentInjectedFrames::TopFrame,
        webkit::UserScriptInjectionTime::End,
        &[],
        &[],
    );
    content_manager.add_script(&extraction_script);
    content_manager.register_script_message_handler("atlas_bridge", None);

    let settings = webkit::Settings::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15")
        .enable_webaudio(true)
        .enable_webgl(true)
        .enable_media_stream(true)
        .enable_developer_extras(true)
        .build();

    let filter_path_str = std::env::temp_dir().to_string_lossy().to_string();
    let filter_manager = webkit::UserContentFilterStore::new(&filter_path_str);
    let filter_json = r#"
    [
        {"trigger": {"url-filter": ".*(?:doubleclick\\.net|googleadservices\\.com|ads\\.youtube\\.com).*"}, "action": {"type": "block"}},
        {"trigger": {"url-filter": ".*(?:amazon-adsystem\\.com|adnxs\\.com|taboola\\.com).*"}, "action": {"type": "block"}}
    ]
    "#;
    let filter_path = std::env::temp_dir().join("atlas_adblock.json");
    std::fs::write(&filter_path, filter_json).unwrap();
    
    let content_manager_clone = content_manager.clone();
    filter_manager.save_from_file(
        "adblock",
        &gtk::gio::File::for_path(&filter_path),
        None::<&gtk::gio::Cancellable>,
        move |result| {
            if let Ok(filter) = result {
                content_manager_clone.add_filter(&filter);
            }
        }
    );

    let tab_view = adw::TabView::new();
    let tab_bar = adw::TabBar::builder()
        .view(&tab_view)
        .autohide(false)
        .build();

    let header_bar = adw::HeaderBar::new();
    let url_entry = gtk::Entry::builder()
        .placeholder_text("Search the web or enter address")
        .hexpand(true)
        .max_width_chars(50)
        .build();

    let new_tab_btn = gtk::Button::from_icon_name("tab-new-symbolic");
    let home_btn = gtk::Button::from_icon_name("go-home-symbolic");
    let back_btn = gtk::Button::from_icon_name("go-previous-symbolic");
    let fwd_btn = gtk::Button::from_icon_name("go-next-symbolic");
    let reload_btn = gtk::Button::from_icon_name("view-refresh-symbolic");
    let toggle_ai_btn = gtk::Button::from_icon_name("view-sidebar-symbolic");

    header_bar.pack_start(&new_tab_btn);
    header_bar.pack_start(&home_btn);
    header_bar.pack_start(&back_btn);
    header_bar.pack_start(&fwd_btn);
    header_bar.pack_start(&reload_btn);
    header_bar.set_title_widget(Some(&url_entry));
    header_bar.pack_end(&toggle_ai_btn);

    let accepted_http: Rc<RefCell<std::collections::HashSet<String>>> = Rc::new(RefCell::new(std::collections::HashSet::new()));

    // Create the initial tab
    let wv_initial = create_tab(&tab_view, &content_manager, &settings, accepted_http.clone(), url_entry.clone());
    
    wv_initial.load_alternate_html(NATIVE_HOMEPAGE, "atlas://home", None);

    let tv_clone = tab_view.clone();
    let cm_clone = content_manager.clone();
    let settings_clone = settings.clone();
    let ue_clone = url_entry.clone();
    let ah_clone = accepted_http.clone();
    new_tab_btn.connect_clicked(move |_| {
        let wv = create_tab(&tv_clone, &cm_clone, &settings_clone, ah_clone.clone(), ue_clone.clone());
        wv.load_alternate_html(NATIVE_HOMEPAGE, "atlas://home", None);
    });

    let tv_clone = tab_view.clone();
    home_btn.connect_clicked(move |_| {
        if let Some(page) = tv_clone.selected_page() {
            if let Ok(wv) = page.child().downcast::<webkit::WebView>() {
                wv.load_alternate_html(NATIVE_HOMEPAGE, "atlas://home", None);
            }
        }
    });

    let tv_clone = tab_view.clone();
    back_btn.connect_clicked(move |_| {
        if let Some(page) = tv_clone.selected_page() {
            if let Ok(wv) = page.child().downcast::<webkit::WebView>() {
                wv.go_back();
            }
        }
    });

    let tv_clone = tab_view.clone();
    fwd_btn.connect_clicked(move |_| {
        if let Some(page) = tv_clone.selected_page() {
            if let Ok(wv) = page.child().downcast::<webkit::WebView>() {
                wv.go_forward();
            }
        }
    });

    let tv_clone = tab_view.clone();
    reload_btn.connect_clicked(move |_| {
        if let Some(page) = tv_clone.selected_page() {
            if let Ok(wv) = page.child().downcast::<webkit::WebView>() {
                wv.reload();
            }
        }
    });

    let tv_clone = tab_view.clone();
    url_entry.connect_activate(move |entry| {
        let text = entry.text().to_string();
        let uri = if text.starts_with("http://") || text.starts_with("https://") {
            text
        } else if text.contains('.') && !text.contains(' ') {
            format!("https://{}", text)
        } else {
            format!("https://duckduckgo.com/?q={}", text)
        };
        
        if let Some(page) = tv_clone.selected_page() {
            if let Ok(wv) = page.child().downcast::<webkit::WebView>() {
                wv.load_uri(&uri);
            }
        }
    });

    let ue_clone = url_entry.clone();
    tab_view.connect_selected_page_notify(move |tv| {
        if let Some(page) = tv.selected_page() {
            if let Ok(wv) = page.child().downcast::<webkit::WebView>() {
                if let Some(uri) = wv.uri() {
                    if uri == "atlas://home" || uri == "atlas://home/" {
                        ue_clone.set_text("");
                    } else {
                        ue_clone.set_text(&uri);
                    }
                } else {
                    ue_clone.set_text("");
                }
            }
        }
    });

    tab_view.connect_close_page(move |tv, page| {
        tv.close_page_finish(page, true);
        true.into()
    });

    let main_content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    main_content.append(&header_bar);
    main_content.append(&tab_bar);
    main_content.append(&tab_view);

    // --- Sidebar ---
    let sidebar_content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .width_request(400)
        .vexpand(true)
        .hexpand(false)
        .css_classes(["sidebar-bg"])
        .build();

    let chat_header = adw::HeaderBar::new();
    chat_header.set_show_end_title_buttons(true);
    chat_header.set_show_start_title_buttons(false);
    let title_label = gtk::Label::new(Some("Atlas AI Agent"));
    title_label.add_css_class("title");
    chat_header.set_title_widget(Some(&title_label));
    
    // Settings Button
    let settings_btn = gtk::Button::from_icon_name("emblem-system-symbolic");
    chat_header.pack_end(&settings_btn);

    let chat_history = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(false)
        .build();
        
    let chat_box = gtk::Box::new(gtk::Orientation::Vertical, 16);
    chat_box.set_margin_top(16);
    chat_box.set_margin_bottom(16);
    chat_box.set_margin_start(16);
    chat_box.set_margin_end(16);
    chat_history.set_child(Some(&chat_box));
    
    let welcome_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    welcome_box.set_halign(gtk::Align::Center);
    let welcome_label = gtk::Label::builder()
        .label("👋 Hello, I am Atlas.\n\nClick the ⚙️ icon above to select your AI Provider and Model. Then ask me to analyze a page or click buttons!")
        .wrap(true)
        .justify(gtk::Justification::Center)
        .css_classes(["dim-label"])
        .build();
    welcome_box.append(&welcome_label);
    chat_box.append(&welcome_box);

    let chat_input = gtk::Entry::builder()
        .placeholder_text("Ask Atlas to analyze or act...")
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(12)
        .build();

    sidebar_content.append(&chat_header);
    sidebar_content.append(&chat_history);
    sidebar_content.append(&chat_input);

    split_view.set_content(Some(&main_content));
    split_view.set_flap(Some(&sidebar_content));
    split_view.set_reveal_flap(false);
    
    // --- Settings UI Dialog ---
    let as_clone_btn = app_settings.clone();
    let win_clone = window.clone();
    settings_btn.connect_clicked(move |_| {
        let pref_win = adw::Window::builder()
            .title("Atlas AI Configuration")
            .default_width(450)
            .default_height(450)
            .modal(true)
            .transient_for(&win_clone)
            .build();
            
        let toolbar_view = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let header = adw::HeaderBar::new();
        toolbar_view.append(&header);
            
        let page = adw::PreferencesPage::new();
        let group = adw::PreferencesGroup::new();
        group.set_title("Provider & Model Configuration");
        
        let provider_row = adw::ComboRow::builder().title("Provider").build();
        let provider_list = gtk::StringList::new(&["Nvidia", "Gemini", "OpenAI"]);
        provider_row.set_model(Some(&provider_list));
        
        let current_prov = as_clone_btn.borrow().provider.clone();
        let idx = match current_prov.as_str() {
            "Gemini" => 1,
            "OpenAI" => 2,
            _ => 0,
        };
        
        let model_row = adw::ComboRow::builder().title("Model").build();
        
        let models_nvidia = ["meta/llama-3.1-405b-instruct", "meta/llama-3.1-70b-instruct", "meta/llama-3.3-70b-instruct"];
        let models_gemini = ["gemini-2.0-flash", "gemini-1.5-pro", "gemini-1.5-flash"];
        let models_openai = ["gpt-4o", "gpt-4o-mini", "o1-mini", "o3-mini"];
        
        let initial_list = match current_prov.as_str() {
            "Gemini" => gtk::StringList::new(&models_gemini),
            "OpenAI" => gtk::StringList::new(&models_openai),
            _ => gtk::StringList::new(&models_nvidia),
        };
        model_row.set_model(Some(&initial_list));
        
        let current_mod = as_clone_btn.borrow().model.clone();
        let current_array = match current_prov.as_str() {
            "Gemini" => models_gemini.to_vec(),
            "OpenAI" => models_openai.to_vec(),
            _ => models_nvidia.to_vec(),
        };
        if let Some(m_idx) = current_array.iter().position(|&x| x == current_mod) {
            model_row.set_selected(m_idx as u32);
        }
        
        provider_row.set_selected(idx);
        
        let key_row = adw::ActionRow::builder()
            .title("API Key")
            .build();
        let key_entry = gtk::PasswordEntry::builder()
            .text(&as_clone_btn.borrow().api_key)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .show_peek_icon(true)
            .build();
        key_row.add_suffix(&key_entry);
            
        group.add(&provider_row);
        group.add(&model_row);
        group.add(&key_row);
        page.add(&group);
        
        let apply_btn = gtk::Button::builder()
            .label("Apply Settings")
            .css_classes(["suggested-action"])
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .build();
            
        let box_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);
        box_layout.append(&page);
        box_layout.append(&apply_btn);
        
        toolbar_view.append(&box_layout);
        pref_win.set_content(Some(&toolbar_view));
        
        let mr_update = model_row.clone();
        provider_row.connect_selected_notify(move |row| {
            let list = match row.selected() {
                1 => gtk::StringList::new(&["gemini-2.0-flash", "gemini-1.5-pro", "gemini-1.5-flash"]),
                2 => gtk::StringList::new(&["gpt-4o", "gpt-4o-mini", "o1-mini", "o3-mini"]),
                _ => gtk::StringList::new(&["meta/llama-3.1-405b-instruct", "meta/llama-3.1-70b-instruct", "meta/llama-3.3-70b-instruct"]),
            };
            mr_update.set_model(Some(&list));
        });
        
        let as_save = as_clone_btn.clone();
        let pr_save = provider_row.clone();
        let mr_save = model_row.clone();
        let kr_save = key_entry.clone();
        let pw_save = pref_win.clone();
        
        apply_btn.connect_clicked(move |_| {
            let mut s = as_save.borrow_mut();
            s.provider = match pr_save.selected() {
                1 => "Gemini".to_string(),
                2 => "OpenAI".to_string(),
                _ => "Nvidia".to_string(),
            };
            
            if let Some(item) = mr_save.selected_item() {
                if let Ok(string_obj) = item.downcast::<gtk::StringObject>() {
                    s.model = string_obj.string().to_string();
                }
            }
            s.api_key = kr_save.text().to_string();
            save_settings(&s);
            pw_save.destroy();
        });
        
        pref_win.present();
    });

    let latest_context: Rc<RefCell<Option<PageContext>>> = Rc::new(RefCell::new(None));
    let lc_clone = latest_context.clone();

    content_manager.connect_script_message_received(Some("atlas_bridge"), move |_manager, message| {
        if let Some(js_val) = message.to_json(0) {
            let json_str = js_val.to_string();
            if let Ok(unquoted) = serde_json::from_str::<String>(&json_str) {
                 if let Ok(context) = serde_json::from_str::<PageContext>(&unquoted) {
                     *lc_clone.borrow_mut() = Some(context);
                 }
            }
        }
    });

    let sv_clone = split_view.clone();
    let tv_clone = tab_view.clone();
    toggle_ai_btn.connect_clicked(move |_| {
        let current = sv_clone.reveals_flap();
        sv_clone.set_reveal_flap(!current);
        
        if !current {
            if let Some(page) = tv_clone.selected_page() {
                if let Ok(wv) = page.child().downcast::<webkit::WebView>() {
                    wv.evaluate_javascript(
                        "window.dispatchEvent(new Event('atlas:request_context'));",
                        None,
                        None,
                        None::<&gtk::gio::Cancellable>,
                        |_| {} 
                    );
                }
            }
        }
    });
    
    let chat_box_clone = chat_box.clone();
    let latest_context_ai = latest_context.clone();
    let chat_history_scroll = chat_history.clone();
    let tv_agent_clone = tab_view.clone();
    let app_settings_ai = app_settings.clone();
    
    chat_input.connect_activate(move |entry| {
        let user_prompt = entry.text().to_string();
        if user_prompt.is_empty() {
            return;
        }
        entry.set_text("");
        
        let user_wrapper = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        user_wrapper.set_halign(gtk::Align::End);
        let user_label = gtk::Label::builder()
            .label(&user_prompt)
            .wrap(true)
            .css_classes(["chat-bubble-user"])
            .build();
        user_wrapper.append(&user_label);
        chat_box_clone.append(&user_wrapper);

        let ai_wrapper = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        ai_wrapper.set_halign(gtk::Align::Start);
        let ai_label = gtk::Label::builder()
            .label("Thinking...")
            .wrap(true)
            .css_classes(["chat-bubble-ai"])
            .build();
        ai_wrapper.append(&ai_label);
        chat_box_clone.append(&ai_wrapper);
        
        let adj = chat_history_scroll.vadjustment();
        adj.set_value(adj.upper());
        
        let context_opt = latest_context_ai.borrow().clone();
        let (sender, receiver) = async_channel::unbounded::<String>();
        let ai_label_clone = ai_label.clone();
        let tv_agent_clone2 = tv_agent_clone.clone();
        let chs_clone = chat_history_scroll.clone();
        
        gtk::glib::spawn_future_local(async move {
            let mut full_ai_response = String::new();
            while let Ok(chunk) = receiver.recv().await {
                if chunk.starts_with("[ERROR") {
                     ai_label_clone.set_label(&format!("API Error: {}", chunk));
                     break;
                }
                if chunk == "[DONE]" {
                     // Check if there is a command in the full response after it finishes
                     if full_ai_response.contains("[CLICK: ") && full_ai_response.contains("]") {
                         let start = full_ai_response.find("[CLICK: ").unwrap() + 8;
                         let end = full_ai_response[start..].find(']').unwrap() + start;
                         let selector = &full_ai_response[start..end];
                         
                         let ghost_script = format!(r#"
                         (function() {{
                             let target = document.querySelector('{}');
                             if (!target) return;
                             let rect = target.getBoundingClientRect();
                             let targetX = rect.left + (rect.width / 2);
                             let targetY = rect.top + (rect.height / 2);
                             let cursor = document.getElementById('atlas-ghost-cursor');
                             if (!cursor) {{
                                 cursor = document.createElement('div');
                                 cursor.id = 'atlas-ghost-cursor';
                                 cursor.innerHTML = '<svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M5.5 3.21V20.8c0 .45.54.67.85.35l4.86-4.86a.5.5 0 01.35-.15h6.94c.45 0 .67-.54.35-.85L6.35 2.86a.5.5 0 00-.85.35z" fill="green" stroke="white" stroke-width="1.5"/></svg>';
                                 cursor.style.position = 'fixed';
                                 cursor.style.zIndex = '999999';
                                 cursor.style.pointerEvents = 'none';
                                 cursor.style.left = window.innerWidth + 'px';
                                 cursor.style.top = (window.innerHeight / 2) + 'px';
                                 cursor.style.transition = 'all 0.6s cubic-bezier(0.25, 1, 0.5, 1)';
                                 cursor.style.filter = 'drop-shadow(0px 4px 6px rgba(118, 185, 0, 0.5))';
                                 document.body.appendChild(cursor);
                                 cursor.getBoundingClientRect();
                             }}
                             cursor.style.left = targetX + 'px';
                             cursor.style.top = targetY + 'px';
                             setTimeout(() => {{
                                 let ripple = document.createElement('div');
                                 ripple.style.position = 'fixed';
                                 ripple.style.left = targetX + 'px';
                                 ripple.style.top = targetY + 'px';
                                 ripple.style.width = '20px';
                                 ripple.style.height = '20px';
                                 ripple.style.borderRadius = '50%';
                                 ripple.style.backgroundColor = 'rgba(118, 185, 0, 0.6)';
                                 ripple.style.transform = 'translate(-50%, -50%) scale(1)';
                                 ripple.style.transition = 'all 0.4s ease-out';
                                 ripple.style.zIndex = '999998';
                                 ripple.style.pointerEvents = 'none';
                                 document.body.appendChild(ripple);
                                 setTimeout(() => {{
                                     ripple.style.transform = 'translate(-50%, -50%) scale(4)';
                                     ripple.style.opacity = '0';
                                 }}, 10);
                                 target.click();
                                 setTimeout(() => ripple.remove(), 400);
                             }}, 650);
                         }})();
                         "#, selector);
                         
                         if let Some(page) = tv_agent_clone2.selected_page() {
                             if let Ok(wv) = page.child().downcast::<webkit::WebView>() {
                                 wv.evaluate_javascript(&ghost_script, None, None, None::<&gtk::gio::Cancellable>, |_| {});
                             }
                         }
                     }
                     break;
                }

                full_ai_response.push_str(&chunk);
                let current_text = ai_label_clone.text().to_string();
                let new_text = if current_text == "Thinking..." {
                    chunk
                } else {
                    current_text + &chunk
                };
                ai_label_clone.set_label(&new_text);
                
                let adj2 = chs_clone.vadjustment();
                adj2.set_value(adj2.upper());
            }
        });

        let sender_clone = sender.clone();
        let curr_settings = app_settings_ai.borrow().clone();
        
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                if curr_settings.api_key.is_empty() {
                    let _ = sender_clone.send("Please configure your API key in Settings (⚙️).".to_string()).await;
                    let _ = sender_clone.send("[DONE]".to_string()).await;
                    return;
                }

                let mut config = OpenAIConfig::new().with_api_key(&curr_settings.api_key);
                if curr_settings.provider == "Nvidia" {
                    config = config.with_api_base("https://integrate.api.nvidia.com/v1");
                } else if curr_settings.provider == "Gemini" {
                    config = config.with_api_base("https://generativelanguage.googleapis.com/v1beta/openai/");
                }
                let client = Client::with_config(config);
                
                let mut system_prompt = String::from("You are Atlas, an AI integrated deeply into a web browser. ");
                system_prompt.push_str("If the user asks you to click a button or a link, respond with ONLY the exact CSS selector wrapped in the tag [CLICK: selector]. For example, if they want to click a button with id 'submit', respond with exactly: [CLICK: #submit]. ");
                if let Some(ctx) = context_opt {
                    if !ctx.url.starts_with("atlas://") {
                        system_prompt.push_str(&format!("The user is viewing website '{}' at {}. ", ctx.title, ctx.url));
                        if !ctx.highlighted_text.is_empty() {
                             system_prompt.push_str(&format!("User highlighted: '{}'. ", ctx.highlighted_text));
                        }
                        system_prompt.push_str(&format!("Page content:\n{}", ctx.main_content));
                    }
                }
                
                let request = CreateChatCompletionRequestArgs::default()
                    .model(&curr_settings.model)
                    .messages([
                        ChatCompletionRequestSystemMessageArgs::default().content(system_prompt).build().unwrap().into(),
                        ChatCompletionRequestUserMessageArgs::default().content(user_prompt).build().unwrap().into(),
                    ])
                    .build()
                    .unwrap();

                match client.chat().create_stream(request).await {
                    Ok(mut stream) => {
                        while let Some(response) = stream.next().await {
                            match response {
                                Ok(resp) => {
                                    for choice in resp.choices {
                                        if let Some(content) = choice.delta.content {
                                            let _ = sender_clone.send(content).await;
                                        }
                                    }
                                }
                                Err(e) => { let _ = sender_clone.send(format!("[ERROR_STREAM] {:?}", e)).await; }
                            }
                        }
                        let _ = sender_clone.send("[DONE]".to_string()).await;
                    },
                    Err(e) => { let _ = sender_clone.send(format!("[ERROR] {:?}", e)).await; }
                }
            });
        });
    });

    window.set_content(Some(&split_view));
    window.present();
}
