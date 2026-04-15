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

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PageContext {
    url: String,
    title: String,
    highlighted_text: String,
    main_content: String,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    
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

fn build_ui(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Linux Atlas")
        .default_width(1200)
        .default_height(800)
        .build();

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

    let web_view = webkit::WebView::builder()
        .user_content_manager(&content_manager)
        .hexpand(true)
        .vexpand(true)
        .build();
    web_view.load_uri("https://duckduckgo.com");

    let header_bar = adw::HeaderBar::new();
    let url_entry = gtk::Entry::builder()
        .placeholder_text("Search or enter address")
        .hexpand(true)
        .max_width_chars(50)
        .build();

    let back_btn = gtk::Button::from_icon_name("go-previous-symbolic");
    let fwd_btn = gtk::Button::from_icon_name("go-next-symbolic");
    let reload_btn = gtk::Button::from_icon_name("view-refresh-symbolic");
    let toggle_ai_btn = gtk::Button::from_icon_name("view-sidebar-symbolic");

    header_bar.pack_start(&back_btn);
    header_bar.pack_start(&fwd_btn);
    header_bar.pack_start(&reload_btn);
    header_bar.set_title_widget(Some(&url_entry));
    header_bar.pack_end(&toggle_ai_btn);

    let wv_clone = web_view.clone();
    back_btn.connect_clicked(move |_| wv_clone.go_back());
    let wv_clone = web_view.clone();
    fwd_btn.connect_clicked(move |_| wv_clone.go_forward());
    let wv_clone = web_view.clone();
    reload_btn.connect_clicked(move |_| wv_clone.reload());

    let wv_clone = web_view.clone();
    url_entry.connect_activate(move |entry| {
        let text = entry.text();
        let uri = if text.starts_with("http://") || text.starts_with("https://") {
            text.to_string()
        } else {
            format!("https://duckduckgo.com/?q={}", text)
        };
        wv_clone.load_uri(&uri);
    });

    let main_content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    main_content.append(&header_bar);
    main_content.append(&web_view);

    let url_entry_clone = url_entry.clone();
    web_view.connect_uri_notify(move |wv: &webkit::WebView| {
        if let Some(uri) = wv.uri() {
            url_entry_clone.set_text(&uri);
        }
    });

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
    
    // Welcome message
    let welcome_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    welcome_box.set_halign(gtk::Align::Center);
    let welcome_label = gtk::Label::builder()
        .label("👋 Hello, I am Atlas.\n\nI can see the pages you visit. Highlight text and ask me to summarize it, or ask me to click buttons for you!")
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
    let wv_clone = web_view.clone();
    toggle_ai_btn.connect_clicked(move |_| {
        let current = sv_clone.reveals_flap();
        sv_clone.set_reveal_flap(!current);
        
        if !current {
            wv_clone.evaluate_javascript(
                "window.dispatchEvent(new Event('atlas:request_context'));",
                None,
                None,
                None::<&gtk::gio::Cancellable>,
                |_| {} 
            );
        }
    });
    
    let wv_for_agent = web_view.clone();
    let chat_box_clone = chat_box.clone();
    let latest_context_ai = latest_context.clone();
    let chat_history_scroll = chat_history.clone();
    
    chat_input.connect_activate(move |entry| {
        let user_prompt = entry.text().to_string();
        if user_prompt.is_empty() {
            return;
        }
        entry.set_text("");
        
        // User bubble
        let user_wrapper = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        user_wrapper.set_halign(gtk::Align::End);
        let user_label = gtk::Label::builder()
            .label(&user_prompt)
            .wrap(true)
            .css_classes(["chat-bubble-user"])
            .build();
        user_wrapper.append(&user_label);
        chat_box_clone.append(&user_wrapper);

        // AI bubble
        let ai_wrapper = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        ai_wrapper.set_halign(gtk::Align::Start);
        let ai_label = gtk::Label::builder()
            .label("Thinking...")
            .wrap(true)
            .css_classes(["chat-bubble-ai"])
            .build();
        ai_wrapper.append(&ai_label);
        chat_box_clone.append(&ai_wrapper);
        
        // Scroll to bottom
        let adj = chat_history_scroll.vadjustment();
        adj.set_value(adj.upper());
        
        let context_opt = latest_context_ai.borrow().clone();
        let (sender, receiver) = async_channel::unbounded::<String>();
        let ai_label_clone = ai_label.clone();
        let wv_for_agent_clone = wv_for_agent.clone();
        
        gtk::glib::spawn_future_local(async move {
            while let Ok(chunk) = receiver.recv().await {
                if chunk == "[ERROR]" {
                     ai_label_clone.set_label("Failed to reach Nvidia API. Check your .env key.");
                     break;
                }
                
                if chunk.starts_with("[CLICK: ") && chunk.ends_with("]") {
                     let selector = chunk.trim_start_matches("[CLICK: ").trim_end_matches("]");
                     
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
                     
                     wv_for_agent_clone.evaluate_javascript(
                         &ghost_script,
                         None,
                         None,
                         None::<&gtk::gio::Cancellable>,
                         |_| {}
                     );
                     
                     ai_label_clone.set_label(&format!("I clicked the element: {}", selector));
                     continue;
                }

                let current_text = ai_label_clone.text().to_string();
                let new_text = if current_text == "Thinking..." {
                    chunk
                } else {
                    current_text + &chunk
                };
                ai_label_clone.set_label(&new_text);
            }
        });

        let sender_clone = sender.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let api_key = env::var("NVIDIA_API_KEY").unwrap_or_default();
                if api_key == "nvapi-XXXXXX" || api_key.is_empty() {
                    let _ = sender_clone.send("[ERROR]".to_string()).await;
                    return;
                }

                let config = OpenAIConfig::new()
                    .with_api_key(api_key)
                    .with_api_base("https://integrate.api.nvidia.com/v1");
                
                let client = Client::with_config(config);
                
                let mut system_prompt = String::from("You are Atlas, an AI integrated deeply into a web browser. ");
                system_prompt.push_str("If the user asks you to click a button or a link, respond ONLY with the exact CSS selector wrapped in the tag [CLICK: selector]. For example, if they want to click a button with id 'submit', respond with exactly: [CLICK: #submit]. ");
                if let Some(ctx) = context_opt {
                    system_prompt.push_str(&format!(
                        "The user is viewing the website '{}' at {}. ",
                        ctx.title, ctx.url
                    ));
                    if !ctx.highlighted_text.is_empty() {
                         system_prompt.push_str(&format!(
                             "The user highlighted this text: '{}'. Focus your answer on this. ",
                             ctx.highlighted_text
                         ));
                    }
                    system_prompt.push_str(&format!(
                        "\nHere is the page content:\n{}",
                        ctx.main_content
                    ));
                }

                let request = CreateChatCompletionRequestArgs::default()
                    .model("meta/llama-3.1-405b-instruct")
                    .messages([
                        ChatCompletionRequestSystemMessageArgs::default()
                            .content(system_prompt)
                            .build().unwrap().into(),
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(user_prompt)
                            .build().unwrap().into(),
                    ])
                    .build()
                    .unwrap();

                match client.chat().create(request).await {
                    Ok(response) => {
                        if let Some(choice) = response.choices.first() {
                            if let Some(content) = &choice.message.content {
                                 let _ = sender_clone.send(content.to_string()).await;
                            }
                        }
                    },
                    Err(e) => {
                        println!("API Error: {:?}", e);
                        let _ = sender_clone.send("[ERROR]".to_string()).await;
                    }
                }
            });
        });
    });

    window.set_content(Some(&split_view));
    window.present();
}
