use dioxus::prelude::*;
use dioxus::document::eval;

#[component]
pub fn LogsTab(logs: Signal<String>) -> Element {
    use_effect(move || {
        // Access logs to trigger effect when logs update
        let _ = logs.read();
        
        // Scroll the live logs container to the bottom
        let _ = eval("
            setTimeout(() => {
                let el = document.getElementById('live-logs-container');
                if (el) {
                    el.scrollTop = el.scrollHeight;
                }
            }, 50);
        ");
    });

    rsx! {
        div { class: "flex flex-col gap-4 h-full w-full overflow-hidden",
            div {
                h2 { class: "text-2xl font-bold text-slate-100", "Live Notification Logs" }
                p { class: "text-sm text-slate-400 mt-1", "View real-time event updates and background agent activities." }
            }

            // Shell Terminal Viewer
            div {
                id: "live-logs-container",
                class: "flex-1 bg-slate-950 border border-slate-850 rounded-2xl p-6 font-mono text-sm leading-relaxed text-slate-300 overflow-y-auto flex flex-col gap-1.5 scrollbar-thin scrollbar-thumb-slate-800 scrollbar-track-transparent select-text",
                if logs.read().is_empty() {
                    div { class: "text-slate-500 italic py-8 text-center",
                        "Terminal is idle. Waiting for logs..."
                    }
                } else {
                    for line in logs.read().lines() {
                        div { class: "whitespace-pre-wrap py-0.5",
                            if line.contains("ERROR") {
                                span { class: "text-rose-400 font-semibold", "{line}" }
                            } else if line.contains("WARN") {
                                span { class: "text-amber-400 font-semibold", "{line}" }
                            } else {
                                span { class: "text-slate-300", "{line}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
