# Linux Atlas 🌍
A native Linux Wayland/GTK4 contextual AI browser inspired by OpenAI's ChatGPT Atlas. 
Built in **Rust** using **WebKitGTK** and **libadwaita**.

## Features
*   **Native GUI:** A beautiful GTK4 interface utilizing `libadwaita` for a polished, modern Linux look. Features a functional browser pane and a sleek sidebar that slides in and out for the AI chat.
*   **Context Extractor:** A custom JavaScript bridge automatically reads the DOM structure (up to 4000 characters to save tokens), the page title, the URL, and whatever text you highlight with your mouse.
*   **Nvidia LLM Brain:** Connects to Nvidia's API (`meta/llama-3.1-405b-instruct`) completely asynchronously using Tokio, streaming responses back into the GTK UI without freezing the window.
*   **The "Agentic" Ghost Cursor:** If you ask the AI to click something, it generates an SVG cursor that visually animates to the button, shows a ripple effect, and then triggers a trusted `click()` event in the DOM.

## Installation & Setup
1. **Dependencies:** You will need GTK4, WebKitGTK 6.0, and Rust installed.
   On Fedora: `sudo dnf install -y webkitgtk6.0-devel javascriptcoregtk6.0-devel`
2. **API Key:** Create a `.env` file in the root directory and add your Nvidia API key:
   `NVIDIA_API_KEY=nvapi-YOUR_KEY_HERE`
3. **Run:** Execute `cargo run` to start the application.

## Credits & License
This project is licensed under the **GPL V3**. 
Concept inspired by OpenAI's "ChatGPT Atlas". 
Built by redminote11tech
