#include "app_shell.h"

#include <algorithm>
#include <cstring>
#include <ctime>
#include <string>

#include <windows.h>
#include <commdlg.h>
#include <shellapi.h>

#include "imgui.h"

namespace app {

namespace {

std::string format_timestamp(int64_t epoch_seconds) {
    if (epoch_seconds <= 0) return "";
    std::time_t t = static_cast<std::time_t>(epoch_seconds);
    std::tm tm_local{};
#ifdef _WIN32
    localtime_s(&tm_local, &t);
#else
    tm_local = *std::localtime(&t);
#endif
    std::time_t now = std::time(nullptr);
    std::tm tm_now{};
#ifdef _WIN32
    localtime_s(&tm_now, &now);
#else
    tm_now = *std::localtime(&now);
#endif
    char buf[32];
    const bool same_day =
        tm_local.tm_year == tm_now.tm_year && tm_local.tm_yday == tm_now.tm_yday;
    if (same_day) {
        std::strftime(buf, sizeof(buf), "%H:%M", &tm_local);
    } else if (tm_local.tm_year == tm_now.tm_year) {
        std::strftime(buf, sizeof(buf), "%b %d", &tm_local);
    } else {
        std::strftime(buf, sizeof(buf), "%Y-%m-%d", &tm_local);
    }
    return buf;
}

std::string to_std(const rust::String& s) {
    return std::string(s);
}

std::string to_std(const rust::Str& s) {
    return std::string(s);
}

std::string wide_to_utf8(const wchar_t* w) {
    if (!w) return {};
    int len = WideCharToMultiByte(CP_UTF8, 0, w, -1, nullptr, 0, nullptr, nullptr);
    if (len <= 1) return {};
    std::string out(static_cast<size_t>(len - 1), '\0');
    WideCharToMultiByte(CP_UTF8, 0, w, -1, out.data(), len, nullptr, nullptr);
    return out;
}

std::string pick_open_file() {
    wchar_t buf[MAX_PATH * 4] = {};
    OPENFILENAMEW ofn{};
    ofn.lStructSize = sizeof(ofn);
    ofn.hwndOwner   = nullptr;
    ofn.lpstrFilter = L"All files\0*.*\0";
    ofn.lpstrFile   = buf;
    ofn.nMaxFile    = MAX_PATH * 4;
    ofn.Flags       = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
    if (!GetOpenFileNameW(&ofn)) return {};
    return wide_to_utf8(buf);
}

std::string format_size_bytes(int64_t n) {
    const char* units[] = { "B", "KB", "MB", "GB" };
    double v = static_cast<double>(n);
    size_t u = 0;
    while (v >= 1024.0 && u + 1 < sizeof(units) / sizeof(units[0])) {
        v /= 1024.0;
        ++u;
    }
    char out[32];
    if (u == 0) std::snprintf(out, sizeof(out), "%d %s", static_cast<int>(n), units[u]);
    else        std::snprintf(out, sizeof(out), "%.1f %s", v, units[u]);
    return out;
}

bool case_insensitive_contains(const std::string& haystack, const std::string& needle) {
    if (needle.empty()) return true;
    auto it = std::search(
        haystack.begin(), haystack.end(), needle.begin(), needle.end(),
        [](char a, char b) { return std::tolower(static_cast<unsigned char>(a)) ==
                                    std::tolower(static_cast<unsigned char>(b)); });
    return it != haystack.end();
}

}  // namespace

std::unique_ptr<AppShell> AppShell::create(const std::string& data_dir) {
    auto box = whatbubbles::init_app(rust::Str(data_dir));
    auto shell = std::unique_ptr<AppShell>(new AppShell(std::move(box)));
    shell->auth_label = to_std(whatbubbles::auth_state(*shell->state));
    const std::string host = to_std(whatbubbles::relay_host(*shell->state));
    std::strncpy(shell->relay_host_buf.data(), host.c_str(),
                 shell->relay_host_buf.size() - 1);
    if (shell->auth_label != "ready") {
        shell->show_setup = true;
    }
    return shell;
}

void AppShell::push_toast(std::string line) {
    toasts.push_back(std::move(line));
    if (toasts.size() > 5) toasts.erase(toasts.begin());
}

void AppShell::drain_events() {
    auto events = whatbubbles::poll_events(*state);
    for (const auto& ev : events) {
        const std::string kind = to_std(ev.kind);
        if (kind == "auth_state") {
            auth_label = to_std(ev.text);
            status_line = "auth → " + auth_label;
            if (auth_label == "ready") show_setup = false;
        } else if (kind == "error") {
            push_toast("error: " + to_std(ev.text));
        } else if (kind == "warn") {
            push_toast("warn: " + to_std(ev.text));
        } else if (kind == "info") {
            status_line = to_std(ev.text);
        } else if (kind == "message_failed") {
            push_toast("send failed: " + to_std(ev.text));
        } else if (kind == "message_sent" || kind == "message_arrived" ||
                   kind == "chat_created" || kind == "chat_updated") {
            status_line = kind + " " + to_std(ev.chat_guid);
        }
    }
}

void AppShell::draw() {
    drain_events();

    const ImGuiViewport* vp = ImGui::GetMainViewport();
    ImGui::SetNextWindowPos(vp->WorkPos);
    ImGui::SetNextWindowSize(vp->WorkSize);

    const ImGuiWindowFlags root_flags =
        ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoBringToFrontOnFocus |
        ImGuiWindowFlags_MenuBar;

    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(0, 0));
    ImGui::Begin("##whatbubbles_root", nullptr, root_flags);
    ImGui::PopStyleVar();

    draw_menu_bar();
    draw_main_layout();

    ImGui::End();

    if (show_setup)           draw_setup_modal();
    if (show_settings)        draw_settings_modal();
    if (show_about)           draw_about_modal();
    if (show_new_chat)        draw_new_chat_modal();
    if (show_facetime_dialog) draw_facetime_modal();
    if (show_demo_imgui)      ImGui::ShowDemoWindow(&show_demo_imgui);

    draw_toasts();
}

void AppShell::draw_menu_bar() {
    if (!ImGui::BeginMenuBar()) return;
    if (ImGui::BeginMenu("WhatBubbles")) {
        if (ImGui::MenuItem("New chat...", "Ctrl+N"))    show_new_chat = true;
        if (ImGui::MenuItem("FaceTime...", "Ctrl+F"))    show_facetime_dialog = true;
        ImGui::Separator();
        if (ImGui::MenuItem("Settings", "Ctrl+,"))        show_settings = true;
        if (ImGui::MenuItem("Setup / account", nullptr)) show_setup = true;
        ImGui::Separator();
        if (ImGui::MenuItem("About"))                     show_about = true;
        ImGui::EndMenu();
    }
    if (ImGui::BeginMenu("View")) {
        ImGui::MenuItem("Show archived chats", nullptr, &show_archived);
        ImGui::Separator();
        ImGui::MenuItem("ImGui demo", nullptr, &show_demo_imgui);
        ImGui::EndMenu();
    }
    ImGui::Dummy(ImVec2(12, 0));
    ImGui::TextDisabled("auth: %s", auth_label.c_str());
    if (!status_line.empty()) {
        ImGui::Dummy(ImVec2(12, 0));
        ImGui::TextDisabled("%s", status_line.c_str());
    }
    ImGui::EndMenuBar();
}

void AppShell::draw_main_layout() {
    const float sidebar_w = 320.0f;
    const ImVec2 avail = ImGui::GetContentRegionAvail();

    ImGui::BeginChild("##sidebar", ImVec2(sidebar_w, avail.y), true);
    draw_sidebar();
    ImGui::EndChild();

    ImGui::SameLine(0, 0);

    ImGui::BeginChild("##conversation", ImVec2(0, avail.y), true);
    draw_conversation();
    ImGui::EndChild();
}

void AppShell::draw_sidebar() {
    if (ImGui::Button("+ New")) show_new_chat = true;
    ImGui::SameLine();
    ImGui::SetNextItemWidth(-1);
    ImGui::InputTextWithHint("##search", "Search chats", chat_search.data(), chat_search.size());
    ImGui::Separator();

    auto chats = whatbubbles::list_chats(*state, show_archived);
    const std::string query(chat_search.data());

    if (chats.empty()) {
        ImGui::TextDisabled("No conversations yet.");
        return;
    }

    for (const auto& chat : chats) {
        const std::string title = to_std(chat.title);
        const std::string subtitle = to_std(chat.subtitle);
        const std::string guid = to_std(chat.guid);
        if (!query.empty() &&
            !case_insensitive_contains(title, query) &&
            !case_insensitive_contains(subtitle, query) &&
            !case_insensitive_contains(to_std(chat.participant_summary), query)) {
            continue;
        }

        ImGui::PushID(guid.c_str());
        const bool is_selected = guid == selected_chat_guid;
        const ImVec2 cursor = ImGui::GetCursorPos();
        if (ImGui::Selectable("##row", is_selected, ImGuiSelectableFlags_AllowItemOverlap,
                              ImVec2(0, 54))) {
            selected_chat_guid = guid;
            whatbubbles::mark_chat_read(*state, rust::Str(guid));
        }

        ImGui::SetCursorPos(ImVec2(cursor.x + 8, cursor.y + 4));
        if (chat.is_pinned) {
            ImGui::TextColored(ImVec4(0.95f, 0.78f, 0.36f, 1), "[P]");
            ImGui::SameLine();
        }
        if (!chat.is_imessage) {
            ImGui::TextColored(ImVec4(0.45f, 0.78f, 0.52f, 1), "[SMS]");
            ImGui::SameLine();
        }
        if (chat.is_archived) {
            ImGui::TextDisabled("[A]");
            ImGui::SameLine();
        }
        ImGui::TextUnformatted(title.c_str());
        if (chat.unread_count > 0) {
            ImGui::SameLine();
            ImGui::TextColored(ImVec4(0.40f, 0.68f, 1.0f, 1), "(%d)", chat.unread_count);
        }

        ImGui::SetCursorPos(ImVec2(cursor.x + 8, cursor.y + 24));
        ImGui::TextDisabled("%s", subtitle.c_str());

        ImGui::SetCursorPos(ImVec2(cursor.x + 8, cursor.y + 38));
        ImGui::TextDisabled("%s", format_timestamp(chat.last_message_date).c_str());

        if (ImGui::BeginPopupContextItem("chat_ctx")) {
            if (ImGui::MenuItem(chat.is_pinned ? "Unpin" : "Pin")) {
                whatbubbles::pin_chat(*state, rust::Str(guid), !chat.is_pinned, 0);
            }
            if (ImGui::MenuItem(chat.is_archived ? "Unarchive" : "Archive")) {
                whatbubbles::archive_chat(*state, rust::Str(guid), !chat.is_archived);
            }
            if (ImGui::MenuItem("Mark read")) {
                whatbubbles::mark_chat_read(*state, rust::Str(guid));
            }
            ImGui::EndPopup();
        }

        ImGui::PopID();
        ImGui::Separator();
    }
}

void AppShell::draw_conversation() {
    if (selected_chat_guid.empty()) {
        ImGui::Dummy(ImVec2(0, ImGui::GetContentRegionAvail().y * 0.4f));
        ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.65f, 0.65f, 0.65f, 1));
        const char* hint = "Select a conversation to begin";
        const float w = ImGui::CalcTextSize(hint).x;
        ImGui::SetCursorPosX((ImGui::GetContentRegionAvail().x - w) * 0.5f);
        ImGui::TextUnformatted(hint);
        ImGui::PopStyleColor();
        return;
    }

    auto messages = whatbubbles::list_messages(*state, rust::Str(selected_chat_guid), 500);

    ImGui::Text("%s", selected_chat_guid.c_str());
    ImGui::SameLine();
    if (ImGui::SmallButton("Mark read")) {
        whatbubbles::mark_chat_read(*state, rust::Str(selected_chat_guid));
    }
    ImGui::SameLine();
    if (ImGui::SmallButton("FaceTime")) {
        show_facetime_dialog = true;
    }
    ImGui::Separator();

    const ImVec2 avail = ImGui::GetContentRegionAvail();
    const float composer_height = 84.0f;

    ImGui::BeginChild("##msgs", ImVec2(0, avail.y - composer_height), true);
    for (const auto& m : messages) {
        const std::string sender = to_std(m.sender_display_name);
        const std::string text   = to_std(m.text);
        const std::string guid   = to_std(m.guid);

        ImGui::PushID(guid.c_str());

        const float bubble_w = ImGui::GetContentRegionAvail().x * 0.72f;
        ImVec4 color = m.is_from_me ? ImVec4(0.18f, 0.46f, 0.96f, 0.22f)
                                    : ImVec4(0.30f, 0.30f, 0.33f, 0.35f);

        if (m.is_from_me) {
            ImGui::Dummy(ImVec2(ImGui::GetContentRegionAvail().x - bubble_w, 0));
            ImGui::SameLine();
        }

        ImGui::PushStyleColor(ImGuiCol_ChildBg, color);
        ImGui::BeginChild(("##b_" + guid).c_str(), ImVec2(bubble_w, 0),
                          true, ImGuiWindowFlags_AlwaysAutoResize);
        if (!m.is_from_me && !sender.empty()) {
            ImGui::TextColored(ImVec4(0.78f, 0.78f, 0.78f, 1), "%s", sender.c_str());
        }
        if (m.is_unsent) {
            ImGui::TextDisabled("(unsent)");
        } else {
            if (!text.empty()) {
                ImGui::TextWrapped("%s", text.c_str());
            }
            if (m.has_attachments) {
                auto atts = whatbubbles::list_attachments(*state, rust::Str(guid));
                for (const auto& a : atts) {
                    const std::string fn = to_std(a.filename);
                    const std::string mt = to_std(a.mime_type);
                    const std::string lp = to_std(a.local_path);
                    ImGui::Separator();
                    ImGui::Text("[file] %s", fn.c_str());
                    ImGui::TextDisabled("%s · %s", mt.c_str(),
                                        format_size_bytes(a.size_bytes).c_str());
                    if (ImGui::SmallButton("Open")) {
                        std::wstring wpath(lp.begin(), lp.end());
                        ShellExecuteW(nullptr, L"open", wpath.c_str(), nullptr, nullptr, SW_SHOWNORMAL);
                    }
                }
            }
        }
        ImGui::TextDisabled("%s%s", format_timestamp(m.date).c_str(),
                            m.date_edited ? " (edited)" : "");
        ImGui::EndChild();
        ImGui::PopStyleColor();

        if (ImGui::BeginPopupContextItem("msg_ctx")) {
            if (ImGui::MenuItem("React ♥")) whatbubbles::tapback_local(*state, rust::Str(guid), rust::Str("love"));
            if (ImGui::MenuItem("React 👍")) whatbubbles::tapback_local(*state, rust::Str(guid), rust::Str("like"));
            if (ImGui::MenuItem("React 👎")) whatbubbles::tapback_local(*state, rust::Str(guid), rust::Str("dislike"));
            ImGui::Separator();
            if (m.is_from_me && !m.is_unsent) {
                if (ImGui::MenuItem("Edit...")) {
                    std::strncpy(composer_buf.data(), text.c_str(), composer_buf.size() - 1);
                }
                if (ImGui::MenuItem("Unsend")) {
                    whatbubbles::unsend_message_local(*state, rust::Str(guid));
                }
            }
            ImGui::EndPopup();
        }

        ImGui::PopID();
        ImGui::Dummy(ImVec2(0, 2));
    }
    if (messages.empty()) {
        ImGui::Dummy(ImVec2(0, 40));
        ImGui::TextDisabled("No messages yet. Send one below.");
    }
    ImGui::SetScrollHereY(1.0f);
    ImGui::EndChild();

    ImGui::SetNextItemWidth(avail.x - 180);
    const bool submit =
        ImGui::InputTextWithHint("##composer", "iMessage — return to send",
                                 composer_buf.data(), composer_buf.size(),
                                 ImGuiInputTextFlags_EnterReturnsTrue);
    ImGui::SameLine();
    const bool attach_clicked = ImGui::Button("Attach", ImVec2(80, 0));
    ImGui::SameLine();
    const bool clicked = ImGui::Button("Send", ImVec2(-1, 0));
    if (attach_clicked) {
        const std::string path = pick_open_file();
        if (!path.empty()) {
            try {
                whatbubbles::attach_file_local(*state, rust::Str(selected_chat_guid),
                                               rust::Str(std::string(composer_buf.data())),
                                               rust::Str(path));
                composer_buf.fill(0);
            } catch (const std::exception& e) {
                push_toast(std::string("attach: ") + e.what());
            }
        }
    }
    if ((submit || clicked) && composer_buf[0] != '\0') {
        whatbubbles::send_message_local(*state, rust::Str(selected_chat_guid),
                                        rust::Str(std::string(composer_buf.data())));
        composer_buf.fill(0);
    }
}

void AppShell::draw_setup_modal() {
    ImGui::SetNextWindowSize(ImVec2(560, 520), ImGuiCond_FirstUseEver);
    if (!ImGui::Begin("Setup", &show_setup)) {
        ImGui::End();
        return;
    }

    ImGui::TextUnformatted("Account & device");
    ImGui::Separator();
    ImGui::Text("Current state: %s", auth_label.c_str());
    const bool has_cfg = whatbubbles::has_os_config(*state);
    ImGui::Text("Relay pairing: %s",
                has_cfg ? std::string(whatbubbles::os_config_summary(*state)).c_str()
                        : "none");
    ImGui::Dummy(ImVec2(0, 8));

    ImGui::TextUnformatted("Step 1 — device pairing");
    ImGui::TextWrapped(
        "Paste a relay pairing code (typically obtained from the OpenBubbles "
        "setup flow on a paired Mac). WhatBubbles hits the relay's "
        "/api/v1/bridge/get-version-info endpoint to fetch hardware info and "
        "persists an OSConfig to os_config.json.");

    ImGui::TextUnformatted("Relay host");
    ImGui::SetNextItemWidth(-1);
    if (ImGui::InputText("##relayhost", relay_host_buf.data(), relay_host_buf.size(),
                         ImGuiInputTextFlags_EnterReturnsTrue)) {
        try {
            whatbubbles::set_relay_host(*state,
                rust::Str(std::string(relay_host_buf.data())));
            push_toast("relay host saved");
        } catch (const std::exception& e) {
            push_toast(std::string("relay host: ") + e.what());
        }
    }

    ImGui::TextUnformatted("Pairing code");
    ImGui::SetNextItemWidth(-1);
    ImGui::InputTextWithHint("##pair", "paste the code here",
                             setup_pair_code.data(), setup_pair_code.size());

    ImGui::TextUnformatted("Beeper access token (optional)");
    ImGui::SetNextItemWidth(-1);
    ImGui::InputTextWithHint("##beeper", "X-Beeper-Access-Token, if the relay needs one",
                             setup_beeper_token.data(), setup_beeper_token.size());

    if (ImGui::Button("Complete pairing")) {
        try {
            whatbubbles::complete_pairing(*state,
                rust::Str(std::string(setup_pair_code.data())),
                rust::Str(std::string(setup_beeper_token.data())));
            auth_label = to_std(whatbubbles::auth_state(*state));
            push_toast("pairing saved");
        } catch (const std::exception& e) {
            push_toast(std::string("pair: ") + e.what());
        }
    }
    ImGui::SameLine();
    if (ImGui::Button("Clear pairing")) {
        try {
            whatbubbles::clear_pairing(*state);
            auth_label = to_std(whatbubbles::auth_state(*state));
        } catch (const std::exception& e) {
            push_toast(std::string("clear: ") + e.what());
        }
    }

    ImGui::Separator();
    ImGui::TextUnformatted("Step 2 — Apple ID");
    if (!has_cfg) {
        ImGui::TextDisabled("Complete step 1 first.");
    }
    ImGui::BeginDisabled(!has_cfg);

    ImGui::TextUnformatted("Apple ID");
    ImGui::SetNextItemWidth(-1);
    ImGui::InputTextWithHint("##apple", "you@icloud.com",
                             setup_apple_id.data(), setup_apple_id.size());

    ImGui::TextUnformatted("Password");
    ImGui::SetNextItemWidth(-1);
    ImGui::InputTextWithHint("##pw", "app-specific password",
                             setup_password.data(), setup_password.size(),
                             ImGuiInputTextFlags_Password);

    if (ImGui::Button("Sign in")) {
        try {
            whatbubbles::start_apple_id_auth(*state,
                rust::Str(std::string(setup_apple_id.data())),
                rust::Str(std::string(setup_password.data())));
        } catch (const std::exception& e) {
            push_toast(std::string("auth: ") + e.what());
        }
    }

    ImGui::Dummy(ImVec2(0, 8));
    ImGui::TextUnformatted("Two-factor code");
    ImGui::SetNextItemWidth(120);
    ImGui::InputText("##2fa", setup_2fa.data(), setup_2fa.size(),
                     ImGuiInputTextFlags_CharsDecimal);
    ImGui::SameLine();
    if (ImGui::Button("Submit 2FA")) {
        try {
            whatbubbles::submit_two_factor_code(*state,
                rust::Str(std::string(setup_2fa.data())));
        } catch (const std::exception& e) {
            push_toast(std::string("2fa: ") + e.what());
        }
    }

    ImGui::EndDisabled();

    ImGui::Dummy(ImVec2(0, 12));
    ImGui::TextDisabled(
        "Apple ID sign-in routes through APS + anisette + IDS registration — not "
        "yet wired. Next-session scope.");
    ImGui::End();
}

void AppShell::draw_settings_modal() {
    ImGui::SetNextWindowSize(ImVec2(520, 380), ImGuiCond_FirstUseEver);
    if (!ImGui::Begin("Settings", &show_settings)) {
        ImGui::End();
        return;
    }

    ImGui::Text("Core version:   %s", std::string(whatbubbles::core_version()).c_str());
    ImGui::Text("Data directory: %s", std::string(whatbubbles::data_dir(*state)).c_str());
    ImGui::Text("Auth state:     %s", auth_label.c_str());
    ImGui::Separator();

    if (ImGui::CollapsingHeader("Known handles")) {
        auto handles = whatbubbles::list_handles(*state);
        if (handles.empty()) ImGui::TextDisabled("No handles cached yet.");
        for (const auto& h : handles) {
            ImGui::BulletText("%s (%s) — %s",
                              std::string(h.display_name).c_str(),
                              std::string(h.service).c_str(),
                              std::string(h.address).c_str());
        }
    }

    if (ImGui::CollapsingHeader("Parity roadmap")) {
        ImGui::BulletText("Attachments (images / video / files)");
        ImGui::BulletText("Typing indicators & read receipts");
        ImGui::BulletText("Stickers");
        ImGui::BulletText("FindMy friends panel");
        ImGui::BulletText("iCloud Shared Albums");
        ImGui::BulletText("FaceTime inbound ringing");
        ImGui::BulletText("SMS / MMS bridge through a paired Mac");
        ImGui::BulletText("Group chat icon / member edits");
        ImGui::TextDisabled("Each item becomes an integration.rs function + a UI panel.");
    }

    if (ImGui::CollapsingHeader("Danger zone")) {
        if (ImGui::Button("Re-seed demo data")) {
            try { whatbubbles::seed_demo(*state); } catch (const std::exception& e) {
                push_toast(std::string("seed: ") + e.what());
            }
        }
    }
    ImGui::End();
}

void AppShell::draw_about_modal() {
    ImGui::SetNextWindowSize(ImVec2(420, 260), ImGuiCond_FirstUseEver);
    if (!ImGui::Begin("About WhatBubbles", &show_about)) { ImGui::End(); return; }
    ImGui::TextUnformatted("WhatBubbles");
    ImGui::TextDisabled("A C++/ImGui client for the OpenBubbles ecosystem.");
    ImGui::Separator();
    ImGui::Text("Core version: %s", std::string(whatbubbles::core_version()).c_str());
    ImGui::Text("Built on Rust + cxx + Dear ImGui + Win32 DX11.");
    ImGui::Separator();
    ImGui::TextWrapped("Based on OpenBubbles (https://github.com/OpenBubbles/openbubbles-app) "
                       "and BlueBubbles (https://github.com/BlueBubblesApp/bluebubbles-app). "
                       "Apache License 2.0. See LICENSE and NOTICE.");
    ImGui::End();
}

void AppShell::draw_new_chat_modal() {
    ImGui::SetNextWindowSize(ImVec2(420, 180), ImGuiCond_FirstUseEver);
    if (!ImGui::Begin("New chat", &show_new_chat)) { ImGui::End(); return; }
    ImGui::TextUnformatted("Recipient (phone number or email)");
    ImGui::SetNextItemWidth(-1);
    ImGui::InputTextWithHint("##target", "+15555550123 or someone@icloud.com",
                             new_chat_target.data(), new_chat_target.size());
    if (ImGui::Button("Start chat")) {
        const std::string addr(new_chat_target.data());
        if (!addr.empty()) {
            push_toast("new-chat stub: need handle-resolution path from rustpush (parked in api.rs)");
        }
    }
    ImGui::End();
}

void AppShell::draw_facetime_modal() {
    ImGui::SetNextWindowSize(ImVec2(420, 160), ImGuiCond_FirstUseEver);
    if (!ImGui::Begin("FaceTime", &show_facetime_dialog)) { ImGui::End(); return; }
    ImGui::TextUnformatted("Call (Apple ID / phone)");
    ImGui::SetNextItemWidth(-1);
    ImGui::InputText("##fttarget", facetime_target.data(), facetime_target.size());
    if (ImGui::Button("Start FaceTime")) {
        try {
            auto link = whatbubbles::start_facetime_call(*state,
                rust::Str(std::string(facetime_target.data())));
            push_toast(std::string("link: ") + std::string(link));
        } catch (const std::exception& e) {
            push_toast(std::string("facetime: ") + e.what());
        }
    }
    ImGui::End();
}

void AppShell::draw_toasts() {
    if (toasts.empty()) return;
    const ImGuiViewport* vp = ImGui::GetMainViewport();
    ImGui::SetNextWindowPos(ImVec2(vp->WorkPos.x + vp->WorkSize.x - 20,
                                   vp->WorkPos.y + vp->WorkSize.y - 20),
                            ImGuiCond_Always, ImVec2(1, 1));
    ImGui::SetNextWindowBgAlpha(0.88f);
    const ImGuiWindowFlags flags =
        ImGuiWindowFlags_NoDecoration | ImGuiWindowFlags_NoMove |
        ImGuiWindowFlags_NoNav | ImGuiWindowFlags_AlwaysAutoResize |
        ImGuiWindowFlags_NoFocusOnAppearing | ImGuiWindowFlags_NoInputs;
    if (ImGui::Begin("##toasts", nullptr, flags)) {
        for (const auto& t : toasts) ImGui::TextUnformatted(t.c_str());
    }
    ImGui::End();
}

}  // namespace app
