#pragma once

#include <array>
#include <memory>
#include <string>
#include <vector>

#include "whatbubbles_cxx/ffi.h"

namespace app {

struct AppShell {
    explicit AppShell(rust::Box<whatbubbles::AppState> s) : state(std::move(s)) {}

    rust::Box<whatbubbles::AppState> state;

    std::string auth_label;
    std::string status_line;
    std::string selected_chat_guid;

    bool show_archived        = false;
    bool show_settings        = false;
    bool show_setup           = false;
    bool show_about           = false;
    bool show_demo_imgui      = false;
    bool show_new_chat        = false;
    bool show_facetime_dialog = false;

    std::array<char, 4096> composer_buf{};
    std::array<char, 256>  setup_apple_id{};
    std::array<char, 256>  setup_password{};
    std::array<char, 16>   setup_2fa{};
    std::array<char, 64>   setup_pair_code{};
    std::array<char, 256>  setup_beeper_token{};
    std::array<char, 256>  relay_host_buf{};
    std::array<char, 128>  chat_search{};
    std::array<char, 256>  new_chat_target{};
    std::array<char, 256>  facetime_target{};

    std::vector<std::string> toasts;

    static std::unique_ptr<AppShell> create(const std::string& data_dir);

    void draw();

private:
    void drain_events();
    void draw_menu_bar();
    void draw_main_layout();
    void draw_sidebar();
    void draw_conversation();
    void draw_setup_modal();
    void draw_settings_modal();
    void draw_about_modal();
    void draw_new_chat_modal();
    void draw_facetime_modal();
    void draw_toasts();

    void push_toast(std::string line);
};

}  // namespace app
