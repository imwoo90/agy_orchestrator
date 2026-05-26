use dioxus::prelude::*;
use crate::models::FeedbackResponse;

#[component]
pub fn FeedbackModal(show_modal: Signal<bool>) -> Element {
    let mut raw_text = use_signal(String::new);
    let mut is_loading = use_signal(|| false);
    let mut response = use_signal(|| None::<Result<FeedbackResponse, String>>);

    if !*show_modal.read() {
        return rsx! {};
    }

    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-sm transition-all duration-300",
            div { class: "w-full max-w-lg bg-slate-900 border border-slate-800/80 rounded-2xl shadow-2xl p-6 flex flex-col gap-4 animate-in fade-in-50 zoom-in-95",
                // Modal Header
                div { class: "flex items-center justify-between border-b border-slate-800 pb-3",
                    h3 { class: "text-lg font-bold text-slate-100 flex items-center gap-2", 
                        span { "💬" }
                        "불편사항 제보 & 기능 제안" 
                    }
                    button {
                        class: "text-slate-400 hover:text-slate-200 transition-colors",
                        onclick: move |_| {
                            show_modal.set(false);
                            raw_text.set(String::new());
                            response.set(None);
                            is_loading.set(false);
                        },
                        "✕"
                    }
                }

                // Modal Content
                if *is_loading.read() {
                    div { class: "flex flex-col items-center justify-center py-12 gap-4",
                        div { class: "h-10 w-10 border-4 border-indigo-500 border-t-transparent rounded-full animate-spin" }
                        div { class: "text-center flex flex-col gap-1.5",
                            p { class: "text-sm font-semibold text-slate-200", "비서 에이전트가 제안을 분석 중입니다..." }
                            p { class: "text-xs text-slate-400", "대충 작성해주신 피드백을 전문 깃허브 이슈 양식으로 다듬고 있습니다." }
                        }
                    }
                } else if let Some(res) = response.read().clone() {
                    match res {
                        Ok(FeedbackResponse::Submitted { title, url }) => rsx! {
                            div { class: "flex flex-col gap-5 py-4",
                                div { class: "flex flex-col items-center gap-2 text-center",
                                    span { class: "text-4xl", "🎉" }
                                    h4 { class: "text-base font-bold text-emerald-400", "제보 제출 완료!" }
                                    p { class: "text-xs text-slate-400", "비서 에이전트가 이슈를 GitHub에 자동으로 즉시 등록했습니다." }
                                }
                                div { class: "bg-slate-950/60 border border-slate-850 rounded-xl p-4 flex flex-col gap-2",
                                    span { class: "text-[10px] font-bold text-slate-500 uppercase tracking-wider", "정제된 이슈 제목" }
                                    p { class: "text-sm text-slate-200 font-semibold leading-relaxed", "{title}" }
                                }
                                div { class: "flex justify-end gap-3 mt-4 border-t border-slate-800 pt-4",
                                    button {
                                        class: "px-4 py-2 text-xs font-semibold rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700/80 transition-all",
                                        onclick: move |_| {
                                            show_modal.set(false);
                                            raw_text.set(String::new());
                                            response.set(None);
                                        },
                                        "닫기"
                                    }
                                    a {
                                        class: "px-4 py-2 text-xs font-semibold rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white shadow-lg shadow-indigo-900/30 transition-all text-center flex items-center justify-center gap-1.5",
                                        href: "{url}",
                                        target: "_blank",
                                        "GitHub에서 확인하기 🔗"
                                    }
                                }
                            }
                        },
                        Ok(FeedbackResponse::PrefilledUrl { title, body, url }) => rsx! {
                            div { class: "flex flex-col gap-4 py-2",
                                div { class: "text-center flex flex-col items-center gap-1.5",
                                    span { class: "text-3xl", "🤖" }
                                    h4 { class: "text-sm font-bold text-slate-200", "비서 에이전트가 이슈 작성을 완료했습니다!" }
                                    p { class: "text-xs text-slate-400 max-w-sm leading-normal", 
                                        "보안 토큰이 설정되어 있지 않아 깃허브에서 직접 등록을 마쳐야 합니다. 비서가 정제한 내용을 전송해 드릴게요!" 
                                    }
                                }

                                div { class: "flex flex-col gap-3 max-h-[220px] overflow-y-auto bg-slate-950/60 border border-slate-850 rounded-xl p-4 scrollbar-thin scrollbar-thumb-slate-800",
                                    div { class: "flex flex-col gap-1",
                                        span { class: "text-[9px] font-bold text-slate-500 uppercase tracking-wider", "정제된 제목" }
                                        p { class: "text-xs text-slate-200 font-semibold", "{title}" }
                                    }
                                    div { class: "flex flex-col gap-1 border-t border-slate-900/80 pt-2",
                                        span { class: "text-[9px] font-bold text-slate-500 uppercase tracking-wider", "정제된 마크다운 본문 미리보기" }
                                        p { class: "text-[11px] text-slate-400 font-mono whitespace-pre-wrap leading-relaxed", "{body}" }
                                    }
                                }

                                div { class: "flex justify-end gap-3 mt-4 border-t border-slate-800 pt-4",
                                    button {
                                        class: "px-4 py-2 text-xs font-semibold rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700/80 transition-all",
                                        onclick: move |_| {
                                            show_modal.set(false);
                                            raw_text.set(String::new());
                                            response.set(None);
                                        },
                                        "취소"
                                    }
                                    a {
                                        class: "px-4 py-2 text-xs font-semibold rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white shadow-lg shadow-indigo-900/30 transition-all text-center flex items-center justify-center gap-1.5",
                                        href: "{url}",
                                        target: "_blank",
                                        onclick: move |_| {
                                            // Close modal shortly after opening browser tab
                                            show_modal.set(false);
                                            raw_text.set(String::new());
                                            response.set(None);
                                        },
                                        "GitHub로 전송하여 완료하기 🚀"
                                    }
                                }
                            }
                        },
                        Err(err) => rsx! {
                            div { class: "flex flex-col gap-4 py-4 text-center items-center",
                                span { class: "text-4xl", "⚠️" }
                                h4 { class: "text-base font-bold text-rose-400", "정제 중 오류 발생" }
                                p { class: "text-xs text-slate-400 max-w-sm leading-relaxed", "{err}" }
                                button {
                                    class: "mt-4 px-4 py-2 text-xs font-semibold rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700/80 transition-all",
                                    onclick: move |_| response.set(None),
                                    "다시 시도"
                                }
                            }
                        }
                    }
                } else {
                    div { class: "flex flex-col gap-4",
                        div { class: "flex flex-col gap-1.5",
                            label { class: "text-xs font-semibold text-slate-400", "비서에게 제안할 피드백 (대충 적어도 괜찮습니다)" }
                            textarea {
                                class: "w-full bg-slate-950/85 border border-slate-800 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 rounded-xl px-3.5 py-3 text-sm text-slate-100 placeholder-slate-600 outline-none transition-all h-32 resize-none leading-relaxed",
                                placeholder: "예: 로그 창 스크롤이 최하단으로 자동 이동하면 좋겠어요.\n예: 설치 스크립트 실행할 때 권한 오류가 나네요.",
                                value: "{raw_text}",
                                oninput: move |e| raw_text.set(e.value())
                            }
                        }

                        div { class: "flex justify-end gap-3 mt-4 border-t border-slate-800 pt-4",
                            button {
                                class: "px-4 py-2 text-xs font-semibold rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 border border-slate-700/80 transition-all",
                                onclick: move |_| {
                                    show_modal.set(false);
                                    raw_text.set(String::new());
                                    response.set(None);
                                },
                                "취소"
                            }
                            button {
                                class: "px-4 py-2 text-xs font-semibold rounded-lg bg-indigo-600 hover:bg-indigo-500 text-white shadow-lg shadow-indigo-900/30 transition-all",
                                onclick: move |_| async move {
                                    if raw_text.read().trim().is_empty() {
                                        return;
                                    }
                                    is_loading.set(true);
                                    let input_text = raw_text.read().clone();
                                    match crate::submit_feedback_fn(input_text).await {
                                        Ok(res) => response.set(Some(Ok(res))),
                                        Err(e) => response.set(Some(Err(format!("비서 에이전트 통신 실패: {}", e)))),
                                    }
                                    is_loading.set(false);
                                },
                                "AI 비서에게 피드백 정제 위임 🤖"
                            }
                        }
                    }
                }
            }
        }
    }
}
